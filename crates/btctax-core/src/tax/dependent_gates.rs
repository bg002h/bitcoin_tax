//! ★★★ **T7 / `SPEC_interview.md` R6 — THE DEPENDENT GATES**: the *Who Qualifies as Your Dependent*
//! flowchart (`design/forms/extract/i1040gi--2025.txt:1447-1812`), transcribed one field per
//! condition and walked one row at a time.
//!
//! ## Why a registry and a walk, and not a predicate
//!
//! The flowchart is five numbered steps whose edges are stated in the instruction's own sentences —
//! *"Yes. Go to Step 2. No. Go to Step 4."*, *"Yes. See Married person, later."*, *"No. STOP — You
//! can't claim any dependents."*. Every one of those is an edge a filer follows, so every one is a
//! field on [`Dependent`] and an entry in [`DEPENDENT_GATES`]. Compressing them into a §152 predicate
//! is the shape `CLAUDE.md` forbids: the dropped term becomes invisible once the lines are gone.
//!
//! ## ★★ Liveness is DERIVED FROM THE WALK, never listed beside it
//!
//! A gate is live for a row **iff the walk demands it** ([`DependentGateQuestion::live`] delegates to
//! [`walk_dependent`]). There is no per-entry liveness predicate, so there is no second copy of the
//! flowchart to drift: a Step 4 gate is live for a row exactly when Step 1 sent that row to Step 4,
//! because that is where the walk asks for it.
//!
//! **Per-row liveness uses the I-4 emulation, not a widened seam** (R6, §10's freeze). The form
//! seam's `Field.live` is `fn(&ReturnInputs) -> bool` with no row, so a dependent-gate `Field` is
//! `live: |_| true` and its `get` returns absent when the gate is not live *for that row* — which the
//! renderer already treats as hidden — exactly the pattern `registries.rs` already uses. The
//! registry keeps its own `live(&ReturnInputs, row)`, which is all `screen_inputs` and
//! `interview_state` need, because they loop the rows themselves.
//!
//! ## ★★ The blocks, and why liveness opens a whole STEP at a time
//!
//! The walk demands a step's conditions **as a block** before it evaluates that step's question,
//! because the instruction states them as a block (*"A qualifying child is your… AND was… AND who
//! didn't… AND who isn't… AND who lived…"*) and only then asks *"1. Do you have a child who meets the
//! conditions to be your qualifying child?"*. Opening one gate at a time would also make
//! `income answer`'s sweep loop as deep as the flowchart, which its `MAX_SWEEPS` guard is not for.
//!
//! ## What is NOT here
//!
//! Printing. Rows (5), (6) and (7) of the TY2025+ Dependents grid are filled by task **T8**, which
//! reads [`DependentVerdict`] — that is why the verdict is a public enum rather than a private
//! branch. Nothing in this module changes what any year emits.

use crate::tax::provenance::DependentGate;
use crate::tax::questions::{Durability, QuestionId};
use crate::tax::return_inputs::{Dependent, ReturnInputs};
use crate::tax::tables::FullReturnParams;
use crate::tax::types::FilingStatus;
use std::borrow::Cow;
use time::Date;

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 1. The named rules the flowchart exits to
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// *Qualifying child of more than one person* — `i1040gi--2025.txt:1967`.
pub const RULE_QC_OF_MORE_THAN_ONE_PERSON: &str =
    "Qualifying child of more than one person (Form 1040 instructions, i1040gi--2025.txt:1967) — \
     \"only one person can claim the child as a qualifying child\" for the child tax credit, head of \
     household, the dependent-care credit and exclusion, and the earned income credit. The tie-break \
     rules are in that section and in Pub. 501; btctax will not choose between two claimants";

/// *Married person* — `i1040gi--2025.txt:1945`.
pub const RULE_MARRIED_PERSON: &str =
    "Married person (Form 1040 instructions, i1040gi--2025.txt:1945) — \"If the person is married and \
     files a joint return, you can't claim that person as your dependent.\" If the person is married \
     but does not file a joint return, or files one only to claim a refund of withheld income tax or \
     estimated tax paid, the instruction sends you back to Step 2 question 3 (a qualifying child) or \
     Step 4 question 4 (a qualifying relative)";

/// The three multi-page rules Step 4's support condition points at —
/// `i1040gi--2025.txt:1823` / `:1940` / `:1949`.
pub const RULE_DIVORCED_MULTIPLE_SUPPORT_KIDNAPPED: &str =
    "Children of divorced or separated parents (i1040gi--2025.txt:1823), Multiple support agreements \
     (i1040gi--2025.txt:1949) and Kidnapped child (i1040gi--2025.txt:1940) — each leaves the \
     flowchart for a multi-page rule with its own conditions and, for the first, a Form 8332 the \
     custodial parent signs. btctax does not model any of the three";

/// *Exception to gross income test* — `i1040gi--2025.txt:1897`.
pub const RULE_EXCEPTION_TO_GROSS_INCOME_TEST: &str =
    "Exception to gross income test (i1040gi--2025.txt:1897) — if the person is permanently and \
     totally disabled, \"certain income for services performed at a sheltered workshop may be \
     excluded for this test\" (Pub. 501). btctax does not model that exclusion";

/// The Step 2 / Step 4 STOP for a person who fails the citizenship test.
const EXIT_CANT_CLAIM_THIS_PERSON: &str = "You can't claim this person as a dependent.";
/// The Step 2 / Step 4 STOP for a filer who is themselves claimable.
const EXIT_CANT_CLAIM_ANY_DEPENDENTS: &str = "You can't claim any dependents.";
/// The Step 4 question 1 STOP.
const EXIT_NOT_A_QUALIFYING_RELATIVE: &str =
    "This person is not your qualifying relative, so they are not your dependent — remove the row.";

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 2. The verdict
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The flowchart STOPped, with the instruction's own exit and the rule it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentRefusal {
    /// The gate whose answer took the flowchart to the STOP.
    pub gate: DependentGate,
    /// The instruction's own exit sentence.
    pub exit: &'static str,
    /// The NAMED rule the instruction sends the filer to, with its line cite. Every REFUSE edge in
    /// R6's table names one — that is what makes the refusal followable rather than a brick.
    pub rule: &'static str,
}

/// ★★★ **THE FLOWCHART STOPPED ON A RETURN-LEVEL ANSWER, not on a gate of the row** (seam review
/// M-1).
///
/// Step 2 question 4 / Step 4 question 5 — *"Could you be claimed as a dependent on someone else's
/// return?"* — is about the FILER, and its `Yes` refuses every dependent row at once. Attributing
/// that to a gate on the row sent the filer to the wrong control: the input form anchored
/// `DependentGateRefused { gate: QcRelationship }` on the relationship tri-state, and a filer who
/// followed the anchor and flipped it merely routed the row to Step 4, where the identical refusal
/// fires again with the identical exit. The refusal TEXT was always right; the anchor was not.
///
/// It carries the same `exit` and `rule` a [`DependentRefusal`] does, because the instruction's own
/// sentence and cite are what make a refusal followable — only the thing it points AT differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentQuestionRefusal {
    /// The return-level declaration whose answer stopped the flowchart.
    pub question: QuestionId,
    /// The instruction's own exit sentence.
    pub exit: &'static str,
    /// The NAMED rule the instruction sends the filer to, with its line cite.
    pub rule: &'static str,
}

/// ★★★ **What the flowchart says about one dependent row.**
///
/// Task **T8** reads this to fill rows (5), (6) and (7) of the TY2025+ Dependents grid; T7 only
/// computes it. The three "claimable" arms are the three states row (7) can be in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependentVerdict {
    /// There is no dependent row at that index.
    NoRow,
    /// A gate the walk demanded has no answer, so the chain stops there. **Class (A): it blocks.**
    Unanswered(DependentGate),
    /// The chain waits on a RETURN-LEVEL declaration that carries its own registry refusal —
    /// *"Could you be claimed as a dependent on someone else's return?"* or Step 5's filer-TIN
    /// question. Those refuse through [`crate::tax::questions::FORM_QUESTIONS`], not through a row.
    WaitingOnQuestion(QuestionId),
    /// The flowchart STOPped: this person is not a dependent on this return.
    Refused(DependentRefusal),
    /// The flowchart STOPped on a RETURN-LEVEL answer — see [`DependentQuestionRefusal`]. Separate
    /// from [`Self::Refused`] so the refusal anchors on the question the filer must change, not on a
    /// gate of the row that cannot fix it (seam review M-1).
    RefusedByQuestion(DependentQuestionRefusal),
    /// A dependent, and row (7)'s *"Child tax credit"* box.
    ChildTaxCredit,
    /// A dependent, and row (7)'s *"Credit for other dependents"* box.
    CreditForOtherDependents,
    /// A dependent, and **neither** row-(7) box — the credit is forgone, not refused
    /// (`i1040gi--2025.txt:1650-1653`, `:1752-1754`).
    NoCreditBox,
}

impl DependentVerdict {
    /// Is this row a claimable dependent (whatever its credit column)?
    #[must_use]
    pub fn is_claimable(&self) -> bool {
        matches!(
            self,
            DependentVerdict::ChildTaxCredit
                | DependentVerdict::CreditForOtherDependents
                | DependentVerdict::NoCreditBox
        )
    }
}

/// The result of walking one row: what the flowchart concluded, and **which gates it demanded on the
/// way** — the latter being the only definition of liveness in this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentWalk {
    pub verdict: DependentVerdict,
    /// A bitset over `DependentGate::ALL` indices. Twenty-one gates, so a `u32` is exact and the walk
    /// allocates nothing.
    demanded: u32,
}

impl DependentWalk {
    /// Did the walk ask for this gate on this row? **This is `live`.**
    #[must_use]
    pub fn demands(&self, gate: DependentGate) -> bool {
        self.demanded & (1u32 << gate_index(gate)) != 0
    }
    /// Every gate the walk demanded, in `DependentGate::ALL` order.
    #[must_use]
    pub fn demanded_gates(&self) -> Vec<DependentGate> {
        DependentGate::ALL
            .iter()
            .copied()
            .filter(|g| self.demands(*g))
            .collect()
    }
}

/// The index of `gate` in `DependentGate::ALL` — derived from the array, never typed beside it.
fn gate_index(gate: DependentGate) -> usize {
    DependentGate::ALL
        .iter()
        .position(|g| *g == gate)
        .expect("DependentGate::ALL is total over the enum (provenance.rs pins it)")
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 3. The walk
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The walk's private cursor.
struct Walk<'a> {
    ri: &'a ReturnInputs,
    d: &'a Dependent,
    demanded: u32,
}

impl<'a> Walk<'a> {
    fn demand(&mut self, gate: DependentGate) {
        self.demanded |= 1u32 << gate_index(gate);
    }
    /// The first DEMANDED gate this row has not answered, in `DependentGate::ALL` order.
    fn first_unanswered(&self) -> Option<DependentGate> {
        DependentGate::ALL.iter().copied().find(|g| {
            self.demanded & (1u32 << gate_index(*g)) != 0 && !gate_is_answered(self.d, *g)
        })
    }
    fn done(self, verdict: DependentVerdict) -> DependentWalk {
        DependentWalk {
            verdict,
            demanded: self.demanded,
        }
    }
    fn yes(&self, gate: DependentGate) -> bool {
        gate_bool(self.d, gate) == Some(true)
    }
    fn no(&self, gate: DependentGate) -> bool {
        gate_bool(self.d, gate) == Some(false)
    }
}

/// ★★★ **THE FLOWCHART, walked for one row.**
///
/// `row` indexes `ri.header.dependents`. An out-of-range row demands nothing and returns
/// [`DependentVerdict::NoRow`], so every caller can loop indices without a bounds dance.
#[must_use]
pub fn walk_dependent(ri: &ReturnInputs, row: usize) -> DependentWalk {
    let Some(d) = ri.header.dependents.get(row) else {
        return DependentWalk {
            verdict: DependentVerdict::NoRow,
            demanded: 0,
        };
    };
    let mut w = Walk { ri, d, demanded: 0 };
    use DependentGate as G;

    // ── STEP 1 — Do You Have a Qualifying Child? (`i1040gi--2025.txt:1485-1529`) ─────────────────
    //
    // The instruction states every condition and only then asks its question, so the whole block is
    // demanded at once. `DateOfBirth` is in it because the age test is a Step 1 CONDITION and Step 1
    // has no *unknown* edge (`:1525-1529`).
    for g in [
        G::DateOfBirth,
        G::QcRelationship,
        G::YoungerThanYouOrSpouse,
        G::FullTimeStudent,
        G::PermanentlyAndTotallyDisabled,
        G::ProvidedOverHalfOwnSupport,
        G::FilingJointReturn,
        G::LivedWithYouOverHalfYear,
    ] {
        w.demand(g);
    }
    // The instruction's own second limb, and only where the first opened it: *"Who isn't filing a
    // joint return for 2025 **or** is filing a joint return for 2025 only to claim a refund…"*
    if w.yes(G::FilingJointReturn) {
        w.demand(G::JointReturnOnlyToClaimRefund);
    }
    // Row (5)(b) is printed *under* row (5)(a) — "(a) Yes … (b) And in the U.S." — so the form's own
    // nesting decides when it is asked.
    if w.yes(G::LivedWithYouOverHalfYear) {
        w.demand(G::LivedWithYouInUs);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }

    let is_qualifying_child = w.yes(G::QcRelationship)
        && age_test(d, ri.tax_year)
        && w.no(G::ProvidedOverHalfOwnSupport)
        && (w.no(G::FilingJointReturn) || w.yes(G::JointReturnOnlyToClaimRefund))
        && w.yes(G::LivedWithYouOverHalfYear);

    if is_qualifying_child {
        step2(w)
    } else {
        // *"1. Do you have a child who meets the conditions to be your qualifying child? … No. Go to
        //  Step 4."* (`:1525-1529`.)
        step4(w)
    }
}

/// **STEP 2 — Is Your Qualifying Child Your Dependent?** (`i1040gi--2025.txt:1535-1590`.)
///
/// The Step 1 CAUTION (`:1519-1521`) is consulted here rather than inside Step 1's block, because it
/// is conditioned on the person BEING a qualifying child: a row that Step 1 sent to Step 4 is a
/// qualifying child of nobody, and Step 4 has its own `qualifying_child_of_any_taxpayer` gate.
fn step2(mut w: Walk<'_>) -> DependentWalk {
    use DependentGate as G;
    for g in [
        G::QualifyingChildOfAnotherPerson,
        G::CitizenNationalResidentOrCanadaMexico,
        G::Married,
    ] {
        w.demand(g);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    if w.yes(G::QualifyingChildOfAnotherPerson) {
        return w.done(refused(
            G::QualifyingChildOfAnotherPerson,
            "This child meets the conditions to be the qualifying child of another person.",
            RULE_QC_OF_MORE_THAN_ONE_PERSON,
        ));
    }
    if w.no(G::CitizenNationalResidentOrCanadaMexico) {
        return w.done(refused(
            G::CitizenNationalResidentOrCanadaMexico,
            EXIT_CANT_CLAIM_THIS_PERSON,
            "Exception to citizen test (i1040gi--2025.txt:1889-1896) covers an adopted child only; \
             Pub. 519 defines U.S. national and U.S. resident alien",
        ));
    }
    if w.yes(G::Married) {
        return w.done(refused(
            G::Married,
            EXIT_CANT_CLAIM_THIS_PERSON,
            RULE_MARRIED_PERSON,
        ));
    }
    // Questions 3 and 4 are about the RETURN, not the row: *"3. Are you filing a joint return for
    // 2025?"* (computed from the filing status) and *"4. Could you be claimed as a dependent on
    // someone else's 2025 tax return?"* (the existing return-level declaration).
    match could_you_be_claimed(w.ri) {
        ClaimedTaxpayer::FilingJointly | ClaimedTaxpayer::No => step3(w),
        ClaimedTaxpayer::Yes => w.done(refused_by_question(
            QuestionId::DependentTaxpayer,
            EXIT_CANT_CLAIM_ANY_DEPENDENTS,
            "Step 2 question 4 / Step 4 question 5 (i1040gi--2025.txt:1571-1589) — someone who can \
             be claimed as a dependent can claim no dependents of their own",
        )),
        ClaimedTaxpayer::Unanswered => {
            w.done(DependentVerdict::WaitingOnQuestion(QuestionId::DependentTaxpayer))
        }
    }
}

/// **STEP 3 — Does Your Qualifying Child Qualify You for the Child Tax Credit or Credit for Other
/// Dependents?** (`i1040gi--2025.txt:1592-1656`.)
fn step3(mut w: Walk<'_>) -> DependentWalk {
    use DependentGate as G;
    for g in [G::TinIssuedByDueDate, G::CitizenNationalOrResidentAlien] {
        w.demand(g);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    // Both STOPs print the same sentence, and NEITHER is a refusal: the person is still a dependent,
    // the credit column is simply blank (`:1650-1653`).
    if w.no(G::TinIssuedByDueDate) || w.no(G::CitizenNationalOrResidentAlien) {
        return w.done(DependentVerdict::NoCreditBox);
    }
    // *"3. Was the child under age 17 at the end of 2025?"* — computed from the row's own date of
    // birth (`:1607-1616`). No ⇒ the credit for other dependents.
    if !under_17_at_year_end(w.d, w.ri.tax_year) {
        return w.done(DependentVerdict::CreditForOtherDependents);
    }
    w.demand(G::SsnsValidForEmploymentIssuedByDueDate);
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    if w.yes(G::SsnsValidForEmploymentIssuedByDueDate) {
        return w.done(DependentVerdict::ChildTaxCredit);
    }
    // *"No. Go to Step 5."* — the credit-for-other-dependents chain.
    step5(w)
}

/// **STEP 4 — Is Your Qualifying Relative Your Dependent?** (`i1040gi--2025.txt:1658-1735`.)
fn step4(mut w: Walk<'_>) -> DependentWalk {
    use DependentGate as G;
    for g in [
        G::QrRelationshipOrMemberOfHousehold,
        G::QualifyingChildOfAnyTaxpayer,
        G::GrossIncomeUnderLimit,
        G::YouProvidedOverHalfSupport,
        G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
    ] {
        w.demand(g);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    // The four conditions, in the instruction's own order, each with its own exit.
    if w.no(G::QrRelationshipOrMemberOfHousehold) {
        return w.done(refused(
            G::QrRelationshipOrMemberOfHousehold,
            EXIT_NOT_A_QUALIFYING_RELATIVE,
            "The Step 4 relationship list (i1040gi--2025.txt:1662-1679), including \"Any other person \
             (other than your spouse) who lived with you all year as a member of your household if \
             your relationship didn't violate local law\"",
        ));
    }
    if w.yes(G::QualifyingChildOfAnyTaxpayer) {
        return w.done(refused(
            G::QualifyingChildOfAnyTaxpayer,
            EXIT_NOT_A_QUALIFYING_RELATIVE,
            "A qualifying relative is one \"Who wasn't a qualifying child (see Step 1) of any \
             taxpayer for 2025\" (i1040gi--2025.txt:1683-1686). See Pub. 501 for who is not a \
             taxpayer for this purpose",
        ));
    }
    if w.no(G::GrossIncomeUnderLimit) {
        return w.done(refused(
            G::GrossIncomeUnderLimit,
            EXIT_NOT_A_QUALIFYING_RELATIVE,
            RULE_EXCEPTION_TO_GROSS_INCOME_TEST,
        ));
    }
    if w.no(G::YouProvidedOverHalfSupport) {
        return w.done(refused(
            G::YouProvidedOverHalfSupport,
            EXIT_NOT_A_QUALIFYING_RELATIVE,
            RULE_DIVORCED_MULTIPLE_SUPPORT_KIDNAPPED,
        ));
    }
    if w.yes(G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies) {
        return w.done(refused(
            G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
            EXIT_NOT_A_QUALIFYING_RELATIVE,
            RULE_DIVORCED_MULTIPLE_SUPPORT_KIDNAPPED,
        ));
    }
    // Questions 2 and 3 are Step 2's own gates, reused (`:1765-1783`, and the exception at `:1895-1896`
    // names them: "U.S. citizen in Step 2, question 1; Step 3, question 2; Step 4, question 2; and
    // Step 5, question 3").
    for g in [G::CitizenNationalResidentOrCanadaMexico, G::Married] {
        w.demand(g);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    if w.no(G::CitizenNationalResidentOrCanadaMexico) {
        return w.done(refused(
            G::CitizenNationalResidentOrCanadaMexico,
            EXIT_CANT_CLAIM_THIS_PERSON,
            "Exception to citizen test (i1040gi--2025.txt:1889-1896) covers an adopted person only; \
             Pub. 519 defines U.S. national and U.S. resident alien",
        ));
    }
    if w.yes(G::Married) {
        return w.done(refused(
            G::Married,
            EXIT_CANT_CLAIM_THIS_PERSON,
            RULE_MARRIED_PERSON,
        ));
    }
    match could_you_be_claimed(w.ri) {
        ClaimedTaxpayer::FilingJointly | ClaimedTaxpayer::No => step5(w),
        ClaimedTaxpayer::Yes => w.done(refused_by_question(
            QuestionId::DependentTaxpayer,
            EXIT_CANT_CLAIM_ANY_DEPENDENTS,
            "Step 2 question 4 / Step 4 question 5 (i1040gi--2025.txt:1717-1735) — someone who can \
             be claimed as a dependent can claim no dependents of their own",
        )),
        ClaimedTaxpayer::Unanswered => {
            w.done(DependentVerdict::WaitingOnQuestion(QuestionId::DependentTaxpayer))
        }
    }
}

/// **STEP 5 — Does Your Qualifying Relative Qualify You for the Credit for Other Dependents?**
/// (`i1040gi--2025.txt:1737-1812`.) Reached from Step 3 question 4's *No* as well as from Step 4.
fn step5(mut w: Walk<'_>) -> DependentWalk {
    use DependentGate as G;
    // Question 1 is about the FILER, so it is a return-level `FormQuestion`, live iff any dependent
    // row exists. `No` ⇒ "You can't claim the credit for other dependents." (`:1752-1754`.)
    match w.ri.header.filer_tin_issued_by_due_date {
        None => {
            return w.done(DependentVerdict::WaitingOnQuestion(
                QuestionId::FilerTinIssuedByDueDate,
            ))
        }
        Some(false) => return w.done(DependentVerdict::NoCreditBox),
        Some(true) => {}
    }
    // Questions 2 and 3 are the Step 3 gates, reused. On the Step 3 path they are already answered;
    // on the Step 4 path this is where they are asked.
    for g in [G::TinIssuedByDueDate, G::CitizenNationalOrResidentAlien] {
        w.demand(g);
    }
    if let Some(g) = w.first_unanswered() {
        return w.done(DependentVerdict::Unanswered(g));
    }
    if w.no(G::TinIssuedByDueDate) || w.no(G::CitizenNationalOrResidentAlien) {
        return w.done(DependentVerdict::NoCreditBox);
    }
    w.done(DependentVerdict::CreditForOtherDependents)
}

fn refused(gate: DependentGate, exit: &'static str, rule: &'static str) -> DependentVerdict {
    DependentVerdict::Refused(DependentRefusal { gate, exit, rule })
}

/// The same STOP, anchored on the RETURN-level question that caused it (seam review M-1).
fn refused_by_question(
    question: QuestionId,
    exit: &'static str,
    rule: &'static str,
) -> DependentVerdict {
    DependentVerdict::RefusedByQuestion(DependentQuestionRefusal {
        question,
        exit,
        rule,
    })
}

/// *"4. Could you be claimed as a dependent on someone else's 2025 tax return?"* — and the question
/// BEFORE it, *"3. Are you filing a joint return?"*, whose `Yes` claims the person outright.
enum ClaimedTaxpayer {
    /// Step 2 question 3 / Step 4 question 4 answered *Yes* — the flowchart claims and moves on.
    FilingJointly,
    No,
    Yes,
    /// The return-level declaration has not been answered; it refuses through `FORM_QUESTIONS`.
    Unanswered,
}

fn could_you_be_claimed(ri: &ReturnInputs) -> ClaimedTaxpayer {
    if ri.filing_status == FilingStatus::Mfj {
        return ClaimedTaxpayer::FilingJointly;
    }
    match ri.header.can_be_claimed_as_dependent_taxpayer {
        None => ClaimedTaxpayer::Unanswered,
        Some(true) => ClaimedTaxpayer::Yes,
        Some(false) => ClaimedTaxpayer::No,
    }
}

/// ★★★ **Step 1's AGE TEST**, from `i1040gi--2025.txt:1487-1499` — the three limbs the instruction
/// prints, joined by its own *"or"*:
///
/// > *"Under age 19 at the end of 2025 and younger than you (or your spouse if filing jointly) **or**
/// > Under age 24 at the end of 2025, a full-time student (defined later), and younger than you (or
/// > your spouse if filing jointly) **or** Any age and permanently and totally disabled (defined
/// > later)."*
///
/// ★ *"younger than you"* is the row's own gate, never a comparison against the taxpayer's date of
///   birth: that DOB is a lawfully declinable class-(B) skippable, and no Step 1 predicate may depend
///   on a declinable value (R6/I4).
///
/// ★ The age boundary is the IRS's own January-1 convention
///   ([`crate::tax::return_1040::considered_age_at_year_end`], `i1040gi--2025.txt:3945-3951`).
#[must_use]
pub fn age_test(d: &Dependent, tax_year: i32) -> bool {
    let Some(dob) = d.date_of_birth else {
        return false;
    };
    let age = crate::tax::return_1040::considered_age_at_year_end(dob, tax_year);
    let younger = d.younger_than_you_or_spouse == Some(true);
    (age < 19 && younger)
        || (age < 24 && d.full_time_student == Some(true) && younger)
        || d.permanently_and_totally_disabled == Some(true)
}

/// *"3. Was the child under age 17 at the end of 2025?"* (`i1040gi--2025.txt:1607`), on the same
/// January-1 convention. A row with no date of birth never reaches this — Step 1 demands the date.
#[must_use]
pub fn under_17_at_year_end(d: &Dependent, tax_year: i32) -> bool {
    d.date_of_birth
        .is_some_and(|dob| crate::tax::return_1040::considered_age_at_year_end(dob, tax_year) < 17)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// 4. The registry
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The value shape of a [`DependentGateQuestion`] — the same split [`crate::tax::questions::SkippableQuestion`]
/// makes, for the same reason: one registry, two answer shapes, and the accessor that does not apply
/// to a gate's `kind` returns `None` / is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    YesNo,
    /// [`DependentGate::DateOfBirth`] alone. It is in this registry rather than beside it so
    /// `DEPENDENT_GATES` stays TOTAL over `DependentGate::ALL` — a completeness check against a
    /// hand-written exemption list is the shape this repo keeps getting wrong.
    Date,
}

/// One dependent gate: the prompt in the instruction's own words, the cite, the accessors, and the
/// refusal detail. Liveness is **not** a field — see [`Self::live`].
pub struct DependentGateQuestion {
    /// The identity T1 minted for `answer_log` (`AnswerKey::DependentGate { ssn_hash, gate }`).
    pub gate: DependentGate,
    /// The prompt, phrased as the INSTRUCTION phrases it.
    pub prompt: &'static str,
    /// ★★★ **R6/M7 — the prompt that must QUOTE a year-package figure.**
    ///
    /// `Some` ⇒ this gate cannot be STATED until the year's [`FullReturnParams`] arrive, so R12 lists
    /// it as *waiting for the year package* rather than as blocking: a prompt that cannot state the
    /// figure asks the filer to derive it. Being a property of the entry, `waiting` is DERIVED here
    /// rather than looked up in a second table keyed by hand.
    pub prompt_from_params: Option<fn(&FullReturnParams, i32) -> String>,
    /// The instruction context a surface shows beside the prompt — for row (5)(a), the *Exception to
    /// time lived with you* verbatim, because the exception is part of the condition.
    pub help: &'static str,
    /// Where the prompt comes from, as `file:lines`.
    pub cite: &'static str,
    pub kind: GateKind,
    /// The yes/no on file (`None` for the `Date` kind).
    pub get: fn(&Dependent) -> Option<bool>,
    /// Record a yes/no (a no-op for the `Date` kind).
    pub set: fn(&mut Dependent, bool),
    /// The date on file (`None` for the `YesNo` kinds).
    pub get_date: fn(&Dependent) -> Option<Date>,
    /// Record a date (a no-op for the `YesNo` kinds).
    pub set_date: fn(&mut Dependent, Date),
    /// ★ **UN-ANSWER** — write the underlying leaf back to `None`. `set`/`set_date` can only write a
    /// definite answer, so the form seam's own `clear` path (review I-1 / spec §5.7 M-6) needs this,
    /// and so does any fixture that must return a row to *never asked*.
    pub clear: fn(&mut Dependent),
    /// The FULL refusal detail for a live, unanswered gate — the cite and the remedy, never derived
    /// from the prompt (the same rule [`crate::tax::questions::FormQuestion::unanswered_detail`] states).
    pub unanswered_detail: &'static str,
    /// §G-15 — whether this answer survives into the next tax year.
    pub durability: Durability,
    /// ★★★ **THE ANSWER THE FLOWCHART'S OWN CONDITION REQUIRES for this row to be claimed as a
    /// qualifying child** — `None` exactly for the one [`GateKind::Date`] gate.
    ///
    /// Declared per gate rather than inferred, for the reason
    /// [`crate::tax::questions::FormQuestion::neutral`] states in terms: polarity is knowledge a new
    /// entry can silently get wrong, and it used to live as a hard-coded `matches!` in `testonly.rs`.
    ///
    /// ★★ **And it is CHECKED, not asserted**: `every_gate_at_its_claim_path_answer_reaches_the_ctc_edge`
    ///      builds a row from this column alone and requires the walk to reach
    ///      [`DependentVerdict::ChildTaxCredit`], so a wrong polarity here reds rather than quietly
    ///      steering a fixture down the qualifying-RELATIVE branch.
    pub claim_path: Option<bool>,
}

impl DependentGateQuestion {
    /// ★★★ **LIVENESS, and there is only one copy of it**: the walk demands this gate on this row.
    #[must_use]
    pub fn live(&self, ri: &ReturnInputs, row: usize) -> bool {
        walk_dependent(ri, row).demands(self.gate)
    }
    /// Can this gate's prompt be stated at all without the year's package?
    #[must_use]
    pub fn needs_params(&self) -> bool {
        self.prompt_from_params.is_some()
    }
    /// ★★★ **The words PUT TO THE FILER** — the same contract
    /// [`crate::tax::questions::FormQuestion::prompt_text`] carries, because
    /// [`crate::tax::provenance::record_answer`] hashes what was SHOWN and R10.3 treats a changed
    /// hash as unanswered. A params-quoting gate therefore re-asks for free when the figure moves.
    ///
    /// ★ With `params: None` on a params-quoting gate this returns the FIGURELESS fallback, which is
    ///   a label for the *waiting* item and must never be put to a filer as a question.
    #[must_use]
    pub fn prompt_text(
        &self,
        ri: &ReturnInputs,
        params: Option<&FullReturnParams>,
    ) -> Cow<'static, str> {
        match (self.prompt_from_params, params) {
            (Some(render), Some(p)) => Cow::Owned(render(p, ri.tax_year)),
            _ => Cow::Borrowed(self.prompt),
        }
    }
}

/// Read one gate off a row as a yes/no (`None` for [`DependentGate::DateOfBirth`]).
#[must_use]
pub fn gate_bool(d: &Dependent, gate: DependentGate) -> Option<bool> {
    (entry(gate).get)(d)
}

/// Has this row answered `gate`, whatever its kind?
#[must_use]
pub fn gate_is_answered(d: &Dependent, gate: DependentGate) -> bool {
    let q = entry(gate);
    match q.kind {
        GateKind::YesNo => (q.get)(d).is_some(),
        GateKind::Date => (q.get_date)(d).is_some(),
    }
}

/// The registry entry for `gate`. Total by construction — `every_gate_has_exactly_one_entry` pins it.
#[must_use]
pub fn entry(gate: DependentGate) -> &'static DependentGateQuestion {
    DEPENDENT_GATES
        .iter()
        .find(|q| q.gate == gate)
        .expect("DEPENDENT_GATES is total over DependentGate::ALL")
}

/// *"Who had gross income of less than $5,200 in 2025."* — rendered with the YEAR's own figure.
fn gross_income_prompt(p: &FullReturnParams, tax_year: i32) -> String {
    format!(
        "Did this person have gross income of less than ${limit} in {tax_year}? (Form 1040 \
         instructions, Step 4: \"Who had gross income of less than ${limit} in {tax_year}. If the \
         person was permanently and totally disabled, see Exception to gross income test, later.\")",
        limit = p.qualifying_relative_gross_income_limit,
    )
}

/// ★★★ **THE DEPENDENT GATE REGISTRY** (R6). One entry per [`DependentGate`] variant, in the
/// flowchart's own order; `DependentGate::ALL` is the completeness anchor, so a new variant is a
/// failing KAT until it is transcribed here.
pub const DEPENDENT_GATES: &[DependentGateQuestion] = &[
    // ── Step 1 ──────────────────────────────────────────────────────────────────────────────────
    DependentGateQuestion {
        gate: DependentGate::DateOfBirth,
        prompt: "What is this person's date of birth?",
        prompt_from_params: None,
        help: "Step 1's age test is computed from this date and the tax year, on the IRS's own \
               January-1 convention: \"A child born on January 1, 2008, is considered to be age 18 at \
               the end of 2025.\" Step 1 ends \"Yes. Go to Step 2. No. Go to Step 4.\" with no \
               \"unknown\" edge, so a row whose age test cannot be evaluated is not printed at all.",
        cite: "i1040gi--2025.txt:1487-1499, :3945-3951",
        kind: GateKind::Date,
        get: |_| None,
        set: |_, _| {},
        get_date: |d| d.date_of_birth,
        set_date: |d, v| d.date_of_birth = Some(v),
        clear: |d| d.date_of_birth = None,
        unanswered_detail:
            "a dependent row needs this person's DATE OF BIRTH: the Step 1 age test \
             (i1040gi--2025.txt:1487-1499) is computed from it, and Step 1 has no \"unknown\" edge, so \
             neither §152(c) nor §152(d) can be established without it — run `btctax income answer`",
        // A birth date cannot change. It is the ONE `Durable` gate here.
        durability: Durability::Durable,
        claim_path: None,
    },
    DependentGateQuestion {
        gate: DependentGate::QcRelationship,
        prompt: "Is this person your son, daughter, stepchild, foster child, brother, sister, \
                 stepbrother, stepsister, half brother, half sister, or a descendant of any of them \
                 (for example, your grandchild, niece, or nephew)?",
        prompt_from_params: None,
        help: "Step 1's relationship test. \"No\" is not a refusal — it sends this row to Step 4, \
               where the wider qualifying-relative list is asked instead.",
        cite: "i1040gi--2025.txt:1463-1466",
        kind: GateKind::YesNo,
        get: |d| d.qc_relationship,
        set: |d, v| d.qc_relationship = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.qc_relationship = None,
        unanswered_detail:
            "Step 1 of Who Qualifies as Your Dependent (i1040gi--2025.txt:1463-1466) asks the \
             qualifying-child relationship test for every dependent row — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::YoungerThanYouOrSpouse,
        prompt: "Was this person younger than you (or your spouse if filing jointly) at the end of \
                 the tax year?",
        prompt_from_params: None,
        help: "Part of Step 1's age test: \"Under age 19 at the end of 2025 and younger than you (or \
               your spouse if filing jointly)\" or \"Under age 24 at the end of 2025, a full-time \
               student (defined later), and younger than you (or your spouse if filing jointly)\". It \
               is ASKED rather than computed, because your own date of birth is a question you may \
               lawfully decline.",
        cite: "i1040gi--2025.txt:1488-1489, :1491-1492",
        kind: GateKind::YesNo,
        get: |d| d.younger_than_you_or_spouse,
        set: |d, v| d.younger_than_you_or_spouse = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.younger_than_you_or_spouse = None,
        unanswered_detail:
            "Step 1's age test (i1040gi--2025.txt:1488-1494) asks whether this person was younger \
             than you, or than your spouse on a joint return. btctax will not answer it from your own \
             date of birth, which you may lawfully decline to give — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::FullTimeStudent,
        prompt: "Was this person a full-time student? (\"A full-time student is a child who during \
                 any part of 5 calendar months of the year was enrolled as a full-time student at a \
                 school or took a full-time, on-farm training course given by a school or a state, \
                 county, or local government agency.\")",
        prompt_from_params: None,
        help: "Row (6) of the Dependents section prints this box, and Step 1's age test reads it: \
               \"Under age 24 at the end of 2025, a full-time student (defined later), and younger \
               than you (or your spouse if filing jointly).\" A school \"includes a technical, trade, \
               or mechanical school. It doesn't include an on-the-job training course, correspondence \
               school, or school offering courses only through the Internet.\"",
        cite: "i1040gi--2025.txt:1491-1494, :1933-1939; f1040--2025.txt:48-49",
        kind: GateKind::YesNo,
        get: |d| d.full_time_student,
        set: |d, v| d.full_time_student = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.full_time_student = None,
        unanswered_detail:
            "row (6) of the Dependents section prints a \"Full-time student\" box for every dependent \
             and Step 1's age test reads it (i1040gi--2025.txt:1491-1494) — an unchecked box and an \
             unasked question are the same mark on paper and are not the same statement — run \
             `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::PermanentlyAndTotallyDisabled,
        prompt: "Was this person permanently and totally disabled? (\"A person is permanently and \
                 totally disabled if, at any time in the year, the person can't engage in any \
                 substantial gainful activity because of a physical or mental condition and a doctor \
                 has determined that this condition has lasted or can be expected to last \
                 continuously for at least a year or can be expected to lead to death.\")",
        prompt_from_params: None,
        help: "Row (6) of the Dependents section prints this box, and it is the third limb of Step \
               1's age test: \"Any age and permanently and totally disabled (defined later).\"",
        cite: "i1040gi--2025.txt:1496-1498, :1958-1962; f1040--2025.txt:48-50",
        kind: GateKind::YesNo,
        get: |d| d.permanently_and_totally_disabled,
        set: |d, v| d.permanently_and_totally_disabled = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.permanently_and_totally_disabled = None,
        unanswered_detail:
            "row (6) of the Dependents section prints a \"Permanently and totally disabled\" box for \
             every dependent, and it is the third limb of Step 1's age test \
             (i1040gi--2025.txt:1496-1498) — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::ProvidedOverHalfOwnSupport,
        prompt: "Did this person provide over half of their own support for the tax year? (See Pub. \
                 501.)",
        prompt_from_params: None,
        help: "Step 1 requires a qualifying child to be someone \"Who didn't provide over half of \
               their own support\", so a \"Yes\" here sends this row to Step 4 rather than refusing.",
        cite: "i1040gi--2025.txt:1502",
        kind: GateKind::YesNo,
        get: |d| d.provided_over_half_own_support,
        set: |d, v| d.provided_over_half_own_support = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.provided_over_half_own_support = None,
        unanswered_detail:
            "Step 1 requires a qualifying child to be someone \"Who didn't provide over half of their \
             own support\" (i1040gi--2025.txt:1502) — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::FilingJointReturn,
        prompt: "Is this person filing a joint return for the tax year?",
        prompt_from_params: None,
        help: "Step 1 requires a qualifying child to be someone \"Who isn't filing a joint return for \
               2025 or is filing a joint return for 2025 only to claim a refund of withheld income \
               tax or estimated tax paid (see Pub. 501 for details and examples)\". Answering \"Yes\" \
               opens the refund-only question.",
        cite: "i1040gi--2025.txt:1506-1508",
        kind: GateKind::YesNo,
        get: |d| d.filing_joint_return,
        set: |d, v| d.filing_joint_return = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.filing_joint_return = None,
        unanswered_detail:
            "Step 1 requires a qualifying child to be someone \"Who isn't filing a joint return\" \
             (i1040gi--2025.txt:1506-1508) — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::JointReturnOnlyToClaimRefund,
        prompt: "Is that joint return being filed ONLY to claim a refund of withheld income tax or \
                 estimated tax paid?",
        prompt_from_params: None,
        help: "The second limb of Step 1's joint-return condition: a qualifying child is one \
               \"Who isn't filing a joint return for 2025 or is filing a joint return for 2025 only \
               to claim a refund of withheld income tax or estimated tax paid (see Pub. 501 for \
               details and examples)\", so a refund-only joint return does not disqualify the child.",
        cite: "i1040gi--2025.txt:1506-1508",
        kind: GateKind::YesNo,
        get: |d| d.joint_return_only_to_claim_refund,
        set: |d, v| d.joint_return_only_to_claim_refund = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.joint_return_only_to_claim_refund = None,
        unanswered_detail:
            "you answered that this person is filing a joint return, so Step 1's second limb applies: \
             a joint return filed \"only to claim a refund of withheld income tax or estimated tax \
             paid\" (i1040gi--2025.txt:1506-1508) does not disqualify them — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::LivedWithYouOverHalfYear,
        prompt: "Did this person live with you for more than half of the tax year? (Read the \
                 Exception to time lived with you before answering — temporary absences, and a child \
                 born or died in the year, count.)",
        prompt_from_params: None,
        // ★★★ THE EXCEPTION IS PART OF THE CONDITION, VERBATIM (`i1040gi--2025.txt:1905-1913`, with
        //     the sentence completed from `:1914`). A child born in November whose home was yours for
        //     more than half the time they were alive answers YES here; answering the bare question
        //     "no" routes them to Step 4, prints the credit for OTHER dependents, and under-claims the
        //     larger credit — on the form's own terms, wrongly.
        help: "Exception to time lived with you. Temporary absences by you or the other person for \
               special circumstances, such as school, vacation, business, medical care, military \
               service, or detention in a juvenile facility, count as time the person lived with you. \
               Also see Children of divorced or separated parents, earlier, or Kidnapped child, \
               later. If the person meets all other requirements to be your qualifying child but was \
               born or died in 2025, the person is considered to have lived with you for more than \
               half of 2025 if your home was this person\u{2019}s home for more than half the time \
               the person was alive in 2025.",
        cite: "i1040gi--2025.txt:1512-1516, :1905-1913; f1040--2025.txt:45-46",
        kind: GateKind::YesNo,
        get: |d| d.lived_with_you_over_half_year,
        set: |d, v| d.lived_with_you_over_half_year = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.lived_with_you_over_half_year = None,
        unanswered_detail:
            "row (5)(a) of the Dependents section — \"Check if lived with you more than half of \
             2025\" — is a Step 1 condition (i1040gi--2025.txt:1512-1516), and the Exception to time \
             lived with you (:1905-1913) is part of it — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::LivedWithYouInUs,
        prompt: "And did this person live with you in the United States?",
        prompt_from_params: None,
        help: "Row (5)(b) of the Dependents section — \"(b) And in the U.S.\" — printed under the row \
               (5)(a) box you just answered.",
        cite: "f1040--2025.txt:47",
        kind: GateKind::YesNo,
        get: |d| d.lived_with_you_in_us,
        set: |d, v| d.lived_with_you_in_us = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.lived_with_you_in_us = None,
        unanswered_detail:
            "row (5)(b) of the Dependents section — \"And in the U.S.\" (f1040--2025.txt:47) — is \
             printed under the row (5)(a) box, and an unchecked box is a statement — run \
             `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    // ── Step 2 (reused at Step 4) ───────────────────────────────────────────────────────────────
    DependentGateQuestion {
        gate: DependentGate::QualifyingChildOfAnotherPerson,
        prompt: "Does this child meet the conditions to be a qualifying child of any other person \
                 (other than your spouse if filing jointly) for the tax year?",
        prompt_from_params: None,
        help: "The CAUTION printed beside Step 1: \"If the child meets the conditions to be a \
               qualifying child of any other person (other than your spouse if filing jointly) for \
               2025, see Qualifying child of more than one person, later.\" Only one person may claim \
               the child for the child tax credit, head of household, the dependent-care credit and \
               exclusion, and the earned income credit.",
        cite: "i1040gi--2025.txt:1519-1521, :1967",
        kind: GateKind::YesNo,
        get: |d| d.qualifying_child_of_another_person,
        set: |d, v| d.qualifying_child_of_another_person = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.qualifying_child_of_another_person = None,
        unanswered_detail:
            "the CAUTION beside Step 1 (i1040gi--2025.txt:1519-1521) asks whether this child meets \
             the conditions to be a qualifying child of another person — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::CitizenNationalResidentOrCanadaMexico,
        prompt: "Was this person a U.S. citizen, U.S. national, U.S. resident alien, or a resident of \
                 Canada or Mexico? (See Pub. 519 for the definition of a U.S. national or U.S. \
                 resident alien. If the person was adopted, see Exception to citizen test.)",
        prompt_from_params: None,
        help: "Step 2 question 1, and Step 4 question 2. \"No\" STOPs the flowchart: \"You can't \
               claim this child as a dependent.\"",
        cite: "i1040gi--2025.txt:1540-1543",
        kind: GateKind::YesNo,
        get: |d| d.citizen_national_resident_or_canada_mexico,
        set: |d, v| d.citizen_national_resident_or_canada_mexico = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.citizen_national_resident_or_canada_mexico = None,
        unanswered_detail:
            "Step 2 question 1 / Step 4 question 2 (i1040gi--2025.txt:1540-1543) asks the citizenship \
             or residency test — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::Married,
        prompt: "Was this person married?",
        prompt_from_params: None,
        help: "Step 2 question 2, and Step 4 question 3. \"Yes\" sends you to Married person: \"If \
               the person is married and files a joint return, you can't claim that person as your \
               dependent.\"",
        cite: "i1040gi--2025.txt:1548, :1945",
        kind: GateKind::YesNo,
        get: |d| d.married,
        set: |d, v| d.married = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.married = None,
        unanswered_detail:
            "Step 2 question 2 / Step 4 question 3 (i1040gi--2025.txt:1548) asks whether this person \
             was married — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    // ── Step 3 (reused at Step 5) ───────────────────────────────────────────────────────────────
    DependentGateQuestion {
        gate: DependentGate::TinIssuedByDueDate,
        prompt: "Did this person have an SSN, ITIN, or adoption taxpayer identification number (ATIN) \
                 issued on or before the due date of your return (including extensions)? (Answer \
                 \"Yes\" if you are applying for an ITIN or ATIN for them on or before the due date \
                 of your return (including extensions).)",
        prompt_from_params: None,
        help: "Step 3 question 1, and Step 5 question 2. \"No\" does NOT stop you claiming this \
               person as a dependent — it means \"You can't claim the child tax credit or the credit \
               for other dependents for this child\", so the row (7) boxes stay blank. The due date \
               INCLUDING EXTENSIONS is October 15 for a calendar-year return that filed Form 4868.",
        cite: "i1040gi--2025.txt:1639-1643",
        kind: GateKind::YesNo,
        get: |d| d.tin_issued_by_due_date,
        set: |d, v| d.tin_issued_by_due_date = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.tin_issued_by_due_date = None,
        unanswered_detail:
            "Step 3 question 1 / Step 5 question 2 (i1040gi--2025.txt:1639-1643) asks whether this \
             person's SSN, ITIN or ATIN was issued on or before the due date of your return including \
             extensions — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::CitizenNationalOrResidentAlien,
        prompt: "Was this person a U.S. citizen, U.S. national, or U.S. resident alien? (See Pub. 519 \
                 for the definition of a U.S. national or U.S. resident alien. If the person was \
                 adopted, see Exception to citizen test.)",
        prompt_from_params: None,
        help: "Step 3 question 2, and Step 5 question 3 — NARROWER than the Step 2 test, which also \
               admits a resident of Canada or Mexico. \"No\" means \"You can't claim the child tax \
               credit or the credit for other dependents for this child\"; the person is still your \
               dependent.",
        cite: "i1040gi--2025.txt:1594-1597",
        kind: GateKind::YesNo,
        get: |d| d.citizen_national_or_resident_alien,
        set: |d, v| d.citizen_national_or_resident_alien = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.citizen_national_or_resident_alien = None,
        unanswered_detail:
            "Step 3 question 2 / Step 5 question 3 (i1040gi--2025.txt:1594-1597) asks a NARROWER \
             citizenship test than Step 2's — no Canada or Mexico — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::SsnsValidForEmploymentIssuedByDueDate,
        prompt: "Did you, or your spouse if filing a joint return, and this child have SSNs valid for \
                 employment and issued before the due date of your return (including extensions)? \
                 (See Social Security Number.)",
        prompt_from_params: None,
        help: "Step 3 question 4 — the child tax credit's own SSN test. \"Yes\" checks the \"Child tax \
               credit\" box on row (7); \"No\" sends you to Step 5, which reaches the credit for \
               other dependents instead.",
        cite: "i1040gi--2025.txt:1619-1622",
        kind: GateKind::YesNo,
        get: |d| d.ssns_valid_for_employment_issued_by_due_date,
        set: |d, v| d.ssns_valid_for_employment_issued_by_due_date = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.ssns_valid_for_employment_issued_by_due_date = None,
        unanswered_detail:
            "Step 3 question 4 (i1040gi--2025.txt:1619-1622) decides between the child tax credit and \
             the credit for other dependents for this child — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    // ── Step 4 ──────────────────────────────────────────────────────────────────────────────────
    DependentGateQuestion {
        gate: DependentGate::QrRelationshipOrMemberOfHousehold,
        prompt: "Is this person your son, daughter, stepchild, foster child, or a descendant of any \
                 of them; your brother, sister, half brother, half sister, or a son or daughter of \
                 any of them; your father, mother, or an ancestor or sibling of either of them; your \
                 stepbrother, stepsister, stepfather, stepmother, son-in-law, daughter-in-law, \
                 father-in-law, mother-in-law, brother-in-law, or sister-in-law; or any other person \
                 (other than your spouse) who lived with you all year as a member of your household \
                 if your relationship didn't violate local law?",
        prompt_from_params: None,
        help: "Step 4's relationship list. \"If the person didn't live with you for the required \
               time, see Exception to time lived with you.\" A \"No\" here STOPs the flowchart: this \
               person is neither your qualifying child nor your qualifying relative.",
        cite: "i1040gi--2025.txt:1662-1679",
        kind: GateKind::YesNo,
        get: |d| d.qr_relationship_or_member_of_household,
        set: |d, v| d.qr_relationship_or_member_of_household = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.qr_relationship_or_member_of_household = None,
        unanswered_detail:
            "Step 1 sent this row to Step 4, whose relationship list (i1040gi--2025.txt:1662-1679) \
             must be answered before the person can be claimed — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::QualifyingChildOfAnyTaxpayer,
        prompt: "Was this person a qualifying child (see Step 1) of any taxpayer for the tax year?",
        prompt_from_params: None,
        help: "Step 4 requires a qualifying relative to be someone \"Who wasn't a qualifying child \
               (see Step 1) of any taxpayer\". \"For this purpose, a person isn't a taxpayer if the \
               person isn't required to file a U.S. income tax return and either doesn't file such a \
               return or files only to get a refund of withheld income tax or estimated tax paid.\"",
        cite: "i1040gi--2025.txt:1683-1686",
        kind: GateKind::YesNo,
        get: |d| d.qualifying_child_of_any_taxpayer,
        set: |d, v| d.qualifying_child_of_any_taxpayer = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.qualifying_child_of_any_taxpayer = None,
        unanswered_detail:
            "Step 4 requires a qualifying relative to be someone \"Who wasn't a qualifying child (see \
             Step 1) of any taxpayer\" (i1040gi--2025.txt:1683-1686) — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
    DependentGateQuestion {
        gate: DependentGate::GrossIncomeUnderLimit,
        // ★ The FIGURELESS fallback. It is a LABEL for R12's *waiting* item and is never put to a
        //   filer: `prompt_from_params` is what a surface asks, and it exists on this entry alone.
        prompt: "Did this person have gross income of less than the §152(d)(1)(B) limit for the tax \
                 year? (Form 1040 instructions, Step 4. The year's figure arrives with its tax \
                 package; until then this question cannot be stated.)",
        prompt_from_params: Some(gross_income_prompt),
        // ★★★ **SEAM REVIEW M-5 — NO FIGURE HERE.** The limit is §152(d)(1)(B)'s exemption amount,
        //     republished for every tax year, and it lives in exactly one place:
        //     `FullReturnParams::qualifying_relative_gross_income_limit`. `prompt_from_params`
        //     already renders the YEAR'S figure into the sentence the filer answers, so a table of
        //     years typed here was a second copy that can drift from the params it duplicates — in
        //     the one module whose header states the rule against a list typed beside derived data.
        //     `no_dependent_gate_help_types_a_figure_the_params_carry` holds it.
        help: "Step 4's gross income test. \"If the person was permanently and totally disabled, see \
               Exception to gross income test, later.\" The limit is §152(d)(1)(B)'s exemption \
               amount, republished for every tax year; the question quotes the year's own figure \
               once the tax package has arrived.",
        cite: "i1040gi--2025.txt:1690-1691",
        kind: GateKind::YesNo,
        get: |d| d.gross_income_under_limit,
        set: |d, v| d.gross_income_under_limit = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.gross_income_under_limit = None,
        unanswered_detail:
            "Step 4's gross income test (i1040gi--2025.txt:1690-1691) asks whether this person's \
             gross income was under §152(d)(1)(B)'s limit for the year — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::YouProvidedOverHalfSupport,
        prompt: "Did you provide over half of this person's support for the tax year?",
        prompt_from_params: None,
        help: "Step 4's support test: \"For whom you provided over half of the person's support in \
               2025. But see Children of divorced or separated parents, Multiple support agreements, \
               and Kidnapped child, later.\"",
        cite: "i1040gi--2025.txt:1695-1696",
        kind: GateKind::YesNo,
        get: |d| d.you_provided_over_half_support,
        set: |d, v| d.you_provided_over_half_support = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.you_provided_over_half_support = None,
        unanswered_detail:
            "Step 4's support test (i1040gi--2025.txt:1695-1696) asks whether you provided over half \
             of this person's support — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(true),
    },
    DependentGateQuestion {
        gate: DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
        prompt: "Does any of Children of divorced or separated parents, Multiple support agreements, \
                 or Kidnapped child apply to this person?",
        prompt_from_params: None,
        help: "Step 4's support condition points at all three: \"But see Children of divorced or \
               separated parents, Multiple support agreements, and Kidnapped child, later.\" Each is \
               a multi-page rule with its own conditions — the first needs a Form 8332 signed by the \
               custodial parent — and btctax models none of them, so a \"Yes\" stops here rather than \
               computing a return on a rule it has not read.",
        cite: "i1040gi--2025.txt:1695-1697, :1823, :1940, :1949",
        kind: GateKind::YesNo,
        get: |d| d.divorced_separated_multiple_support_or_kidnapped_rule_applies,
        set: |d, v| d.divorced_separated_multiple_support_or_kidnapped_rule_applies = Some(v),
        get_date: |_| None,
        set_date: |_, _| {},
        clear: |d| d.divorced_separated_multiple_support_or_kidnapped_rule_applies = None,
        unanswered_detail:
            "Step 4's support condition points at Children of divorced or separated parents, Multiple \
             support agreements and Kidnapped child (i1040gi--2025.txt:1695-1697) — btctax models \
             none of the three, so it must ask whether any applies — run `btctax income answer`",
        durability: Durability::PerYear,
        claim_path: Some(false),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::HouseholdHeader;
    use crate::tax::return_refuse::{screen_inputs, RefuseReason};
    use time::macros::date;

    /// A return with one dependent row, every gate blank.
    fn one_dependent(tax_year: i32) -> ReturnInputs {
        ReturnInputs {
            tax_year,
            filing_status: FilingStatus::Single,
            header: HouseholdHeader {
                dependents: vec![Dependent {
                    name: "Kid Example".into(),
                    ssn: "000-00-1111".into(),
                    relationship: "Daughter".into(),
                    ..Default::default()
                }],
                can_be_claimed_as_dependent_taxpayer: Some(false),
                filer_tin_issued_by_due_date: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// The row at the CHILD TAX CREDIT edge — every gate at its declared `claim_path`, a date of
    /// birth that is under 17 and under 19 at the end of `tax_year`.
    fn at_the_ctc_edge(tax_year: i32) -> ReturnInputs {
        let mut ri = one_dependent(tax_year);
        ri.header.dependents[0].date_of_birth =
            Some(Date::from_calendar_date(tax_year - 10, time::Month::June, 1).unwrap());
        // ★ Every RETURN-LEVEL declaration too, so a screen assertion below is attributable to the
        //   gate it perturbs rather than to an unrelated blank.
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        ri
    }

    fn verdict(ri: &ReturnInputs) -> DependentVerdict {
        walk_dependent(ri, 0).verdict
    }

    // ── Registry completeness ────────────────────────────────────────────────────────────────────

    /// ★★★ **`DEPENDENT_GATES` is TOTAL over `DependentGate::ALL`, both directions.** The enum is
    /// the completeness anchor (T1 owns it), so a gate identity with no registry entry — or an entry
    /// for an identity that does not exist — reds here rather than being discovered by a filer.
    #[test]
    fn every_gate_has_exactly_one_entry_and_every_entry_names_a_real_gate() {
        assert_eq!(
            DEPENDENT_GATES.len(),
            DependentGate::ALL.len(),
            "one registry entry per gate identity"
        );
        for g in DependentGate::ALL {
            assert_eq!(
                DEPENDENT_GATES.iter().filter(|q| q.gate == *g).count(),
                1,
                "exactly one DEPENDENT_GATES entry for {g:?}"
            );
        }
        // Exactly one Date gate, and it is the one with no `claim_path`.
        let dates: Vec<_> = DEPENDENT_GATES
            .iter()
            .filter(|q| q.kind == GateKind::Date)
            .map(|q| q.gate)
            .collect();
        assert_eq!(dates, vec![DependentGate::DateOfBirth]);
        for q in DEPENDENT_GATES {
            assert_eq!(
                q.claim_path.is_none(),
                q.kind == GateKind::Date,
                "{:?}: `claim_path` is None exactly for the Date gate",
                q.gate
            );
            assert!(
                !q.prompt.trim().is_empty() && !q.cite.trim().is_empty(),
                "{:?}: every gate carries a prompt and a cite",
                q.gate
            );
        }
    }

    /// ★★★ **SEAM REVIEW M-5 — NO GATE'S `help` TYPES A FIGURE THE PARAMS CARRY.**
    ///
    /// Exactly one gate quotes money, its figure is
    /// `FullReturnParams::qualifying_relative_gross_income_limit`, and `prompt_from_params` renders
    /// it. A table of years typed into `help` beside it is a second copy of derived data — the shape
    /// this module's own header forbids — and it can drift from the params silently, because
    /// nothing reads it. Derived over the whole registry, so a figure typed into any FUTURE gate's
    /// help reds here too.
    #[test]
    fn no_dependent_gate_help_types_a_figure_the_params_carry() {
        for q in DEPENDENT_GATES {
            assert!(
                !q.help.contains('$'),
                "{:?}'s help types a dollar figure: the year's figure comes from \
                 `FullReturnParams`, through `prompt_from_params`, and nowhere else — {:?}",
                q.gate,
                q.help
            );
        }
        // ★ The positive control: the figure IS still reaching the filer, so this is not satisfied
        //   by a registry that stopped saying anything.
        let mut p = crate::tax::testonly::ty2024_params();
        p.qualifying_relative_gross_income_limit = rust_decimal_macros::dec!(5300);
        let asked = entry(DependentGate::GrossIncomeUnderLimit)
            .prompt_text(&one_dependent(2026), Some(&p))
            .into_owned();
        assert!(
            asked.contains("5300"),
            "the words PUT TO THE FILER still quote the year's figure: {asked}"
        );
    }

    /// ★★★ **THE `claim_path` COLUMN IS CHECKED, NOT ASSERTED.** A row built from that column alone
    /// must reach the CTC edge — so a wrong polarity reds here rather than quietly steering every
    /// fixture in the workspace down the qualifying-RELATIVE branch.
    #[test]
    fn every_gate_at_its_claim_path_answer_reaches_the_ctc_edge() {
        let ri = at_the_ctc_edge(2025);
        assert_eq!(verdict(&ri), DependentVerdict::ChildTaxCredit);
        // …and the row really did take the QUALIFYING-CHILD branch, not the relative one.
        let walk = walk_dependent(&ri, 0);
        assert!(walk.demands(DependentGate::SsnsValidForEmploymentIssuedByDueDate));
        assert!(!walk.demands(DependentGate::QrRelationshipOrMemberOfHousehold));
    }

    // ── The truth table, one row per flowchart edge (R6's table) ─────────────────────────────────

    /// ★★★ **THE TRUTH-TABLE KAT — one row per edge of R6's table.**
    ///
    /// Each row starts from the CTC edge, perturbs exactly ONE answer, and asserts the outcome the
    /// instruction gives: Step 4, a named refusal, or the credit column T8 will print. Starting from
    /// a known-good row is what makes each assertion attributable to its own perturbation.
    #[test]
    fn the_flowchart_truth_table() {
        type Edge = (&'static str, fn(&mut ReturnInputs), DependentVerdict);
        let refused = |gate, rule: &'static str| {
            DependentVerdict::Refused(DependentRefusal {
                gate,
                exit: "",
                rule,
            })
        };
        let rows: &[Edge] = &[
            // ── Step 1: each condition's FALSE edge sends the row to Step 4, which then refuses on
            //    its own unanswered relationship question — "Step 4" is exactly what that means here.
            (
                "Step 1 relationship No ⇒ Step 4",
                |ri| ri.header.dependents[0].qc_relationship = Some(false),
                DependentVerdict::Unanswered(DependentGate::QrRelationshipOrMemberOfHousehold),
            ),
            (
                "Step 1 age test fails (not younger) ⇒ Step 4",
                |ri| ri.header.dependents[0].younger_than_you_or_spouse = Some(false),
                DependentVerdict::Unanswered(DependentGate::QrRelationshipOrMemberOfHousehold),
            ),
            (
                "Step 1 provided over half own support Yes ⇒ Step 4",
                |ri| ri.header.dependents[0].provided_over_half_own_support = Some(true),
                DependentVerdict::Unanswered(DependentGate::QrRelationshipOrMemberOfHousehold),
            ),
            (
                "Step 1 joint return and NOT refund-only ⇒ Step 4",
                |ri| {
                    ri.header.dependents[0].filing_joint_return = Some(true);
                    ri.header.dependents[0].joint_return_only_to_claim_refund = Some(false);
                },
                DependentVerdict::Unanswered(DependentGate::QrRelationshipOrMemberOfHousehold),
            ),
            (
                "Step 1 joint return ONLY to claim a refund ⇒ still a qualifying child",
                |ri| {
                    ri.header.dependents[0].filing_joint_return = Some(true);
                    ri.header.dependents[0].joint_return_only_to_claim_refund = Some(true);
                },
                DependentVerdict::ChildTaxCredit,
            ),
            (
                "Step 1 did not live with you over half the year ⇒ Step 4",
                |ri| ri.header.dependents[0].lived_with_you_over_half_year = Some(false),
                DependentVerdict::Unanswered(DependentGate::QrRelationshipOrMemberOfHousehold),
            ),
            (
                "the Step 1 CAUTION ⇒ REFUSE naming Qualifying child of more than one person",
                |ri| ri.header.dependents[0].qualifying_child_of_another_person = Some(true),
                refused(
                    DependentGate::QualifyingChildOfAnotherPerson,
                    RULE_QC_OF_MORE_THAN_ONE_PERSON,
                ),
            ),
            // ── Step 2 ──
            (
                "Step 2 q1 citizenship No ⇒ REFUSE, You can't claim this person as a dependent",
                |ri| ri.header.dependents[0].citizen_national_resident_or_canada_mexico = Some(false),
                refused(
                    DependentGate::CitizenNationalResidentOrCanadaMexico,
                    "Exception to citizen test (i1040gi--2025.txt:1889-1896) covers an adopted \
                     child only; Pub. 519 defines U.S. national and U.S. resident alien",
                ),
            ),
            (
                "Step 2 q2 married Yes ⇒ REFUSE naming Married person",
                |ri| ri.header.dependents[0].married = Some(true),
                refused(DependentGate::Married, RULE_MARRIED_PERSON),
            ),
            // ★ Seam review M-1 — this STOP comes from a RETURN-LEVEL answer, so its verdict names
            //   the QUESTION. Nothing on the row can change it.
            (
                "Step 2 q4 could-you-be-claimed Yes ⇒ REFUSE, You can't claim any dependents",
                |ri| ri.header.can_be_claimed_as_dependent_taxpayer = Some(true),
                DependentVerdict::RefusedByQuestion(DependentQuestionRefusal {
                    question: QuestionId::DependentTaxpayer,
                    exit: "",
                    rule: "Step 2 question 4 / Step 4 question 5 (i1040gi--2025.txt:1571-1589) — \
                           someone who can be claimed as a dependent can claim no dependents of \
                           their own",
                }),
            ),
            (
                "Step 2 q3 filing a joint return ⇒ claimed, and on to Step 3",
                |ri| ri.filing_status = FilingStatus::Mfj,
                DependentVerdict::ChildTaxCredit,
            ),
            // ── Step 3: the credit column ──
            (
                "Step 3 q1 no TIN by the due date ⇒ no credit box (a forgo, not a refusal)",
                |ri| ri.header.dependents[0].tin_issued_by_due_date = Some(false),
                DependentVerdict::NoCreditBox,
            ),
            (
                "Step 3 q2 narrower citizenship No ⇒ no credit box",
                |ri| ri.header.dependents[0].citizen_national_or_resident_alien = Some(false),
                DependentVerdict::NoCreditBox,
            ),
            (
                "Step 3 q3 NOT under 17 ⇒ the credit for other dependents",
                |ri| {
                    ri.header.dependents[0].date_of_birth = Some(date!(2007 - 06 - 01));
                    ri.header.dependents[0].full_time_student = Some(true);
                },
                DependentVerdict::CreditForOtherDependents,
            ),
            (
                "Step 3 q4 SSNs not valid for employment ⇒ Step 5 ⇒ the credit for other dependents",
                |ri| {
                    ri.header.dependents[0].ssns_valid_for_employment_issued_by_due_date =
                        Some(false)
                },
                DependentVerdict::CreditForOtherDependents,
            ),
            // ★★★ **SEAM REVIEW N-1 — THE UNDER-17 JANUARY-1 BOUNDARY, as a PAIR.** Step 3
            //     question 3 is computed from the row's date of birth on the IRS's own convention
            //     (*"a child born on January 1, 2008, is considered to be age 18 at the end of
            //     2025"*, `i1040gi--2025.txt:3945-3951`). The other under-17 row uses a date ten
            //     years back, nowhere near the edge, so the GATE's use of the convention was only
            //     transitively covered by `return_1040`'s own KAT. One day apart, opposite verdicts.
            (
                "Step 3 q3 born January 1 ⇒ considered 17 at the end of 2025 ⇒ ODC, not the CTC",
                |ri| ri.header.dependents[0].date_of_birth = Some(date!(2009 - 01 - 01)),
                DependentVerdict::CreditForOtherDependents,
            ),
            (
                "…and born January 2 ⇒ considered 16 ⇒ the child tax credit",
                |ri| ri.header.dependents[0].date_of_birth = Some(date!(2009 - 01 - 02)),
                DependentVerdict::ChildTaxCredit,
            ),
            // ── Step 5's own question, reached from Step 3 q4's No ──
            (
                "Step 5 q1 the FILER has no TIN by the due date ⇒ no credit box",
                |ri| {
                    ri.header.dependents[0].ssns_valid_for_employment_issued_by_due_date =
                        Some(false);
                    ri.header.filer_tin_issued_by_due_date = Some(false);
                },
                DependentVerdict::NoCreditBox,
            ),
        ];
        for (name, perturb, want) in rows {
            let mut ri = at_the_ctc_edge(2025);
            perturb(&mut ri);
            let got = verdict(&ri);
            match (&got, want) {
                // A refusal is compared on the GATE and the RULE; the exit sentence is asserted
                // separately by `every_refuse_edge_names_its_rule`.
                (DependentVerdict::Refused(g), DependentVerdict::Refused(w)) => assert_eq!(
                    (g.gate, g.rule),
                    (w.gate, w.rule),
                    "{name}: the flowchart took a different exit"
                ),
                // …and a RETURN-LEVEL stop is compared on the QUESTION and the rule (M-1).
                (
                    DependentVerdict::RefusedByQuestion(g),
                    DependentVerdict::RefusedByQuestion(w),
                ) => assert_eq!(
                    (g.question, g.rule),
                    (w.question, w.rule),
                    "{name}: the flowchart took a different exit"
                ),
                _ => assert_eq!(&got, want, "{name}"),
            }
        }
    }

    /// ★★★ **THE STEP 4 CHAIN**, walked from the row Step 1 sent there — the other half of the
    /// truth table, since every Step 4 edge needs the row to be a qualifying RELATIVE first.
    #[test]
    fn the_step_four_truth_table() {
        // A grandparent: not a qualifying child (fails the relationship test), claimed under §152(d).
        let relative = || {
            let mut ri = one_dependent(2025);
            let d = &mut ri.header.dependents[0];
            d.relationship = "Mother".into();
            d.date_of_birth = Some(date!(1950 - 03 - 04));
            d.qc_relationship = Some(false);
            d.younger_than_you_or_spouse = Some(false);
            d.full_time_student = Some(false);
            d.permanently_and_totally_disabled = Some(false);
            d.provided_over_half_own_support = Some(false);
            d.filing_joint_return = Some(false);
            d.lived_with_you_over_half_year = Some(true);
            d.lived_with_you_in_us = Some(true);
            d.qr_relationship_or_member_of_household = Some(true);
            d.qualifying_child_of_any_taxpayer = Some(false);
            d.gross_income_under_limit = Some(true);
            d.you_provided_over_half_support = Some(true);
            d.divorced_separated_multiple_support_or_kidnapped_rule_applies = Some(false);
            d.citizen_national_resident_or_canada_mexico = Some(true);
            d.married = Some(false);
            d.tin_issued_by_due_date = Some(true);
            d.citizen_national_or_resident_alien = Some(true);
            ri
        };
        assert_eq!(
            verdict(&relative()),
            DependentVerdict::CreditForOtherDependents,
            "a qualifying relative who clears Step 5 takes the credit for other dependents"
        );
        // A qualifying relative never reaches the child tax credit's own SSN question.
        assert!(!walk_dependent(&relative(), 0)
            .demands(DependentGate::SsnsValidForEmploymentIssuedByDueDate));

        type Edge = (
            &'static str,
            fn(&mut ReturnInputs),
            DependentGate,
            &'static str,
        );
        let rows: &[Edge] = &[
            (
                "Step 4 relationship No ⇒ REFUSE — not a dependent",
                |ri| ri.header.dependents[0].qr_relationship_or_member_of_household = Some(false),
                DependentGate::QrRelationshipOrMemberOfHousehold,
                "The Step 4 relationship list (i1040gi--2025.txt:1662-1679)",
            ),
            (
                "a qualifying child of any taxpayer ⇒ REFUSE — not a qualifying relative",
                |ri| ri.header.dependents[0].qualifying_child_of_any_taxpayer = Some(true),
                DependentGate::QualifyingChildOfAnyTaxpayer,
                "Who wasn't a qualifying child (see Step 1) of any taxpayer",
            ),
            (
                "gross income over the §152(d)(1)(B) limit ⇒ REFUSE naming the exception",
                |ri| ri.header.dependents[0].gross_income_under_limit = Some(false),
                DependentGate::GrossIncomeUnderLimit,
                "Exception to gross income test",
            ),
            (
                "you did not provide over half the support ⇒ REFUSE naming the three rules",
                |ri| ri.header.dependents[0].you_provided_over_half_support = Some(false),
                DependentGate::YouProvidedOverHalfSupport,
                "Multiple support agreements",
            ),
            (
                "a divorced/multiple-support/kidnapped rule applies ⇒ REFUSE naming all three",
                |ri| {
                    ri.header.dependents[0]
                        .divorced_separated_multiple_support_or_kidnapped_rule_applies = Some(true)
                },
                DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
                "Children of divorced or separated parents",
            ),
            // ★★★ **SEAM REVIEW I-5 — STEP 4 QUESTION 2, THE CITIZEN STOP.** R6's table names this
            //     edge and the instruction states it as its own question (`i1040gi--2025.txt:
            //     1765-1772), but no row covered it: `the_flowchart_truth_table`'s citizen row
            //     starts from the CTC edge, so it exercises STEP 2's arm. Deleting Step 4's whole
            //     `if w.no(CitizenNationalResidentOrCanadaMexico)` block claimed a non-citizen,
            //     non-resident qualifying relative as a dependent — a wrong result on the form's
            //     own terms — with 1315 tests still green.
            //
            // ★ The rule fragment asserted is *"adopted person only"*, which is Step 4's own
            //   wording; Step 2's says *"adopted child only"*. So this row also pins WHICH arm ran.
            (
                "Step 4 q2 citizenship No ⇒ REFUSE — You can't claim this person as a dependent",
                |ri| {
                    ri.header.dependents[0].citizen_national_resident_or_canada_mexico = Some(false)
                },
                DependentGate::CitizenNationalResidentOrCanadaMexico,
                "covers an adopted person only",
            ),
            (
                "Step 4 q3 married Yes ⇒ REFUSE naming Married person",
                |ri| ri.header.dependents[0].married = Some(true),
                DependentGate::Married,
                "Married person",
            ),
        ];
        for (name, perturb, gate, rule_fragment) in rows {
            let mut ri = relative();
            perturb(&mut ri);
            match verdict(&ri) {
                DependentVerdict::Refused(r) => {
                    assert_eq!(
                        r.gate, *gate,
                        "{name}: the wrong gate stopped the flowchart"
                    );
                    assert!(
                        r.rule.contains(rule_fragment),
                        "{name}: the refusal must NAME its rule — got {:?}",
                        r.rule
                    );
                }
                other => panic!("{name}: expected a refusal, got {other:?}"),
            }
        }
        // ★★★ **SEAM REVIEW M-1 — STEP 4 QUESTION 5 anchors on the QUESTION, not on a gate.** It
        //     is the only Step 4 STOP that no answer ON THE ROW can lift, and it used to be
        //     reported as `Refused { gate: QrRelationshipOrMemberOfHousehold }` — which sent the
        //     filer to a control that merely re-routes the row into the same refusal.
        {
            let mut ri = relative();
            ri.header.can_be_claimed_as_dependent_taxpayer = Some(true);
            match verdict(&ri) {
                DependentVerdict::RefusedByQuestion(r) => {
                    assert_eq!(r.question, QuestionId::DependentTaxpayer);
                    assert_eq!(r.exit, "You can't claim any dependents.");
                    assert!(
                        r.rule.contains("can claim no dependents of their own"),
                        "the refusal must NAME its rule — got {:?}",
                        r.rule
                    );
                }
                other => panic!("Step 4 q5 Yes must refuse on the QUESTION, got {other:?}"),
            }
        }
        // ★★★ **SEAM REVIEW I-5 — STEP 4 QUESTION 4, on the relative path.** *"Are you filing a
        //     joint return for 2025?"* is computed from the filing status, and its `Yes` CLAIMS the
        //     person and moves on to Step 5 — it never consults the could-you-be-claimed
        //     declaration. The declaration is blanked here so the Mfj arm is the ONLY thing that
        //     can produce a verdict: without it the walk waits on `DependentTaxpayer`.
        {
            let mut ri = relative();
            ri.header.can_be_claimed_as_dependent_taxpayer = None;
            assert_eq!(
                verdict(&ri),
                DependentVerdict::WaitingOnQuestion(QuestionId::DependentTaxpayer),
                "the premise: with the declaration blank and no joint return, the chain WAITS"
            );
            ri.filing_status = FilingStatus::Mfj;
            assert_eq!(
                verdict(&ri),
                DependentVerdict::CreditForOtherDependents,
                "Step 4 q4 Yes ⇒ claimed, on to Step 5 — the declaration is never reached"
            );
        }
        // ★★★ **SEAM REVIEW I-5 — STEP 5 QUESTION 1 answered *No*, on the RELATIVE path.** The
        //     FILER's own TIN. Only the Step-3-derived path had a row for it, and the two paths
        //     reach Step 5 through different arms.
        {
            let mut ri = relative();
            ri.header.filer_tin_issued_by_due_date = Some(false);
            assert_eq!(
                verdict(&ri),
                DependentVerdict::NoCreditBox,
                "Step 5 q1 No ⇒ \"You can't claim the credit for other dependents\" — a forgo, and \
                 the person is still a dependent"
            );
        }
        // Step 5's own two reused gates, on the RELATIVE path.
        for (name, perturb) in [
            (
                "Step 5 q2 the relative has no TIN by the due date",
                (|ri: &mut ReturnInputs| {
                    ri.header.dependents[0].tin_issued_by_due_date = Some(false)
                }) as fn(&mut ReturnInputs),
            ),
            (
                "Step 5 q3 the relative fails the narrower citizenship test",
                |ri: &mut ReturnInputs| {
                    ri.header.dependents[0].citizen_national_or_resident_alien = Some(false)
                },
            ),
        ] {
            let mut ri = relative();
            perturb(&mut ri);
            assert_eq!(
                verdict(&ri),
                DependentVerdict::NoCreditBox,
                "{name}: a dependent, with no credit box — a forgo, never a refusal"
            );
        }
    }

    // ── The two edges r1 dropped ─────────────────────────────────────────────────────────────────

    /// ★★★ **A CHILD BORN IN NOVEMBER LANDS ON THE CTC EDGE, NOT ON ODC.** The *Exception to time
    /// lived with you* is part of row (5)(a)'s condition — it is quoted verbatim in the gate's own
    /// `help` — so the filer answers *Yes* and the child is a qualifying child. Answering the bare
    /// question *No* routes them to Step 4, passes every qualifying-relative test (a newborn has no
    /// gross income and the filer provided all support) and prints the SMALLER credit: wrong on the
    /// form's own terms, and not even a visible forgo.
    #[test]
    fn a_child_born_in_november_reaches_the_child_tax_credit() {
        let mut ri = at_the_ctc_edge(2026);
        ri.header.dependents[0].date_of_birth = Some(date!(2026 - 11 - 01));
        ri.header.dependents[0].lived_with_you_over_half_year = Some(true);
        assert_eq!(verdict(&ri), DependentVerdict::ChildTaxCredit);

        // ★ The MISROUTE this exists to prevent, shown to be reachable: the same child, answering
        //   the bare question literally, lands on the credit for OTHER dependents.
        let mut misread = at_the_ctc_edge(2026);
        let d = &mut misread.header.dependents[0];
        d.date_of_birth = Some(date!(2026 - 11 - 01));
        d.lived_with_you_over_half_year = Some(false);
        d.qr_relationship_or_member_of_household = Some(true);
        d.qualifying_child_of_any_taxpayer = Some(false);
        d.gross_income_under_limit = Some(true);
        d.you_provided_over_half_support = Some(true);
        d.divorced_separated_multiple_support_or_kidnapped_rule_applies = Some(false);
        assert_eq!(
            verdict(&misread),
            DependentVerdict::CreditForOtherDependents,
            "the misroute is REAL — which is why the exception must be in front of the filer"
        );
    }

    /// ★★★ **A DECLINED TAXPAYER DATE OF BIRTH STILL RESOLVES STEP 1.** *"Younger than you"* is the
    /// row's own gate, never a comparison against the taxpayer's DOB — that DOB is a lawfully
    /// declinable class-(B) skippable, and no Step 1 predicate may depend on a declinable value.
    #[test]
    fn a_declined_taxpayer_date_of_birth_still_resolves_step_one() {
        use crate::tax::provenance::{record_answer, AnswerKey, AnswerState};
        use crate::tax::questions::SkippableId;
        let mut ri = at_the_ctc_edge(2025);
        assert_eq!(
            ri.header.taxpayer.date_of_birth, None,
            "declined ⇒ no value"
        );
        let sk = crate::tax::questions::SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::DobTaxpayer)
            .expect("the taxpayer DOB skippable exists");
        record_answer(
            &mut ri,
            AnswerKey::Skippable(SkippableId::DobTaxpayer),
            sk.prompt,
            date!(2026 - 04 - 15),
            AnswerState::Declined,
        );
        assert_eq!(
            verdict(&ri),
            DependentVerdict::ChildTaxCredit,
            "Step 1 resolves from the ROW's own gate, not from a value the filer declined to give"
        );
    }

    // ── The screen ───────────────────────────────────────────────────────────────────────────────

    fn screened(ri: &ReturnInputs) -> Option<RefuseReason> {
        let p = crate::tax::testonly::ty2024_params();
        let t = crate::tax::testonly::ty2024_table();
        screen_inputs(ri, &t, &p).map(|r| r.reason)
    }

    fn screened_detail(ri: &ReturnInputs) -> String {
        let p = crate::tax::testonly::ty2024_params();
        let t = crate::tax::testonly::ty2024_table();
        screen_inputs(ri, &t, &p)
            .map(|r| r.detail)
            .unwrap_or_default()
    }

    /// ★★★ **EVERY GATE: `None` WHILE LIVE ⇒ `DependentGateUnanswered`, THROUGH `screen_inputs`.**
    ///
    /// Derived from the walk, not from a list: for each gate, a row is primed so the walk demands it
    /// and the gate alone is blanked. `date_of_birth = None` is one of the rows — R6 makes it
    /// required, and this is where that refusal fires, before any print path.
    #[test]
    fn every_live_gate_that_is_blank_refuses_by_name() {
        for q in DEPENDENT_GATES {
            let mut ri = at_the_ctc_edge(2024);
            // Reach the gate: the Step 4 gates need Step 1 to have said "no qualifying child".
            if matches!(
                q.gate,
                DependentGate::QrRelationshipOrMemberOfHousehold
                    | DependentGate::QualifyingChildOfAnyTaxpayer
                    | DependentGate::GrossIncomeUnderLimit
                    | DependentGate::YouProvidedOverHalfSupport
                    | DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies
            ) {
                ri.header.dependents[0].qc_relationship = Some(false);
                crate::tax::testonly::answer_all_dependent_gates(&mut ri);
            }
            if q.gate == DependentGate::JointReturnOnlyToClaimRefund {
                ri.header.dependents[0].filing_joint_return = Some(true);
            }
            (q.clear)(&mut ri.header.dependents[0]);
            assert!(
                q.live(&ri, 0),
                "{:?} must be LIVE in its own scenario, or this row asserts nothing",
                q.gate
            );
            assert_eq!(
                screened(&ri),
                Some(RefuseReason::DependentGateUnanswered {
                    row: 0,
                    gate: q.gate
                }),
                "a blank live {:?} must refuse with its own reason",
                q.gate
            );
            // …and answering it clears THAT refusal.
            crate::tax::testonly::answer_all_dependent_gates(&mut ri);
            assert_ne!(
                screened(&ri),
                Some(RefuseReason::DependentGateUnanswered {
                    row: 0,
                    gate: q.gate
                }),
                "answering {:?} must clear its own refusal",
                q.gate
            );
        }
    }

    /// ★★★ **EVERY REFUSE EDGE NAMES ITS RULE, IN THE TEXT THE FILER IS SHOWN.** A refusal with no
    /// exit is a brick with better prose, so the screen's own detail — not just the walk's struct —
    /// is what is asserted here.
    #[test]
    fn every_refuse_edge_names_its_rule_in_the_screen_detail() {
        type Case = (fn(&mut ReturnInputs), &'static str);
        let cases: &[Case] = &[
            (
                |ri| ri.header.dependents[0].qualifying_child_of_another_person = Some(true),
                "Qualifying child of more than one person",
            ),
            (
                |ri| ri.header.dependents[0].married = Some(true),
                "Married person",
            ),
            (
                |ri| {
                    ri.header.dependents[0].citizen_national_resident_or_canada_mexico = Some(false)
                },
                "You can't claim this person as a dependent.",
            ),
            (
                |ri| ri.header.can_be_claimed_as_dependent_taxpayer = Some(true),
                "You can't claim any dependents.",
            ),
        ];
        for (perturb, needle) in cases {
            let mut ri = at_the_ctc_edge(2024);
            perturb(&mut ri);
            let detail = screened_detail(&ri);
            assert!(
                detail.contains(needle),
                "the refusal must name {needle:?} — got {detail:?}"
            );
            assert!(
                detail.contains("btctax income answer") || detail.contains("Remove the row"),
                "and it must carry an EXIT — got {detail:?}"
            );
            // ★★★ **SEAM REVIEW M-2 — NO RUN OF SPACES.** The detail was one long string literal
            //     whose wrapped lines had no `\` continuation, so the filer read two 22-space runs
            //     mid-sentence. `cargo fmt` does not touch string literals, so nothing else catches
            //     it; this does, for every refuse edge at once.
            assert!(
                !detail.contains("  "),
                "a missing `\\` continuation collapses into a run of spaces: {detail:?}"
            );
        }
        // ★ The same check on the two IDENTITY refusals and on the unanswered detail, so the whole
        //   dependent surface is covered rather than only the STOPs.
        for perturb in [
            (|ri: &mut ReturnInputs| ri.header.dependents[0].ssn = String::new())
                as fn(&mut ReturnInputs),
            |ri: &mut ReturnInputs| {
                let twin = ri.header.dependents[0].clone();
                ri.header.dependents.push(twin);
            },
            |ri: &mut ReturnInputs| ri.header.dependents[0].lived_with_you_over_half_year = None,
        ] {
            let mut ri = at_the_ctc_edge(2024);
            perturb(&mut ri);
            let detail = screened_detail(&ri);
            assert!(!detail.is_empty(), "the premise: this perturbation refuses");
            assert!(
                !detail.contains("  "),
                "a missing `\\` continuation collapses into a run of spaces: {detail:?}"
            );
        }
    }

    /// ★★★ **A `Single` RETURN WITH NO DEPENDENTS ASKS NO GATE AND NO FILER-TIN QUESTION.**
    #[test]
    fn a_return_with_no_dependents_asks_nothing() {
        let mut ri = ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        assert!(ri.header.dependents.is_empty());
        assert_eq!(walk_dependent(&ri, 0).verdict, DependentVerdict::NoRow);
        assert!(
            walk_dependent(&ri, 0).demanded_gates().is_empty(),
            "no row ⇒ no gate is live"
        );
        let filer_tin = crate::tax::questions::FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::FilerTinIssuedByDueDate)
            .expect("the filer-TIN question is registered");
        assert!(
            !(filer_tin.live)(&ri),
            "Step 5 is reached only through a dependent, so the question is not live"
        );
        assert_eq!(screened(&ri), None, "and the return screens clean");
        // …and one row makes both true.
        ri.header.dependents.push(Dependent::default());
        assert!((filer_tin.live)(&ri));
        assert!(!walk_dependent(&ri, 0).demanded_gates().is_empty());
    }

    // ── The params-quoting gate ──────────────────────────────────────────────────────────────────

    /// ★★★ **`gross_income_under_limit` WAITS on a params-less year and BLOCKS once the package
    /// lands — and its prompt QUOTES the year's figure.** A prompt that cannot state the figure asks
    /// the filer to derive it, and a derived answer to a §6065 declaration is what this interview
    /// exists to prevent.
    #[test]
    fn the_params_quoting_gate_waits_then_blocks_and_quotes_the_figure() {
        use crate::tax::interview_state::{interview_state, interview_state_with_params};
        use crate::tax::provenance::AnswerKey;
        // A row Step 1 sent to Step 4, on TY2026 — the year with no `FullReturnParams`.
        let mut ri = one_dependent(2026);
        let d = &mut ri.header.dependents[0];
        d.date_of_birth = Some(date!(1950 - 03 - 04));
        d.qc_relationship = Some(false);
        d.younger_than_you_or_spouse = Some(false);
        d.full_time_student = Some(false);
        d.permanently_and_totally_disabled = Some(false);
        d.provided_over_half_own_support = Some(false);
        d.filing_joint_return = Some(false);
        d.lived_with_you_over_half_year = Some(true);
        d.lived_with_you_in_us = Some(true);
        let key = |ri: &ReturnInputs| AnswerKey::DependentGate {
            ssn_hash: crate::tax::provenance::dependent_ssn_hash(&ri.header.dependents[0].ssn),
            gate: DependentGate::GrossIncomeUnderLimit,
        };
        let k = key(&ri);
        let st = interview_state(&ri);
        assert!(
            st.waiting.iter().any(|w| w.item == k),
            "on a params-less year the gate WAITS: {:?}",
            st.waiting
        );
        assert!(
            !st.blocking.iter().any(|b| b.item == k),
            "…and is never blocking"
        );
        assert_eq!(
            st.waiting
                .iter()
                .find(|w| w.item == k)
                .map(|w| w.waiting_on),
            Some(crate::tax::interview_state::YEAR_PACKAGE),
            "and it names what it waits on"
        );

        // ★ With the package inserted it MOVES — same return, same gate.
        let mut p = crate::tax::testonly::ty2024_params();
        p.qualifying_relative_gross_income_limit = rust_decimal_macros::dec!(5300);
        let st = interview_state_with_params(&ri, &p);
        assert!(
            !st.waiting.iter().any(|w| w.item == k),
            "with params the gate no longer waits"
        );
        let b = st
            .blocking
            .iter()
            .find(|b| b.item == k)
            .expect("…it BLOCKS");
        assert!(
            b.prompt.contains("$5300") && b.prompt.contains("2026"),
            "and its prompt QUOTES the year's own figure: {:?}",
            b.prompt
        );
        // ★ The figureless fallback is not what a filer is asked — it names the missing package.
        let q = entry(DependentGate::GrossIncomeUnderLimit);
        assert!(q.needs_params());
        assert!(
            q.prompt_text(&ri, None)
                .contains("arrives with its tax package"),
            "the fallback says WHY it cannot be asked"
        );
    }

    /// ★★★ **THE SHIPPED FIGURES ARE THE INSTRUCTION'S OWN.** The Step 4 sentence prints the limit,
    /// so the gate's rendered prompt must reproduce the year's figure exactly — TY2024 $5,050
    /// (Rev. Proc. 2023-34 §3.24) and TY2026 $5,300 (Rev. Proc. 2025-32 §4.23).
    #[test]
    fn the_gross_income_limit_matches_the_shipped_params() {
        let q = entry(DependentGate::GrossIncomeUnderLimit);
        let mut ri = one_dependent(2024);
        ri.header.dependents[0].date_of_birth = Some(date!(1950 - 03 - 04));
        let p = crate::tax::testonly::ty2024_params();
        assert_eq!(
            p.qualifying_relative_gross_income_limit,
            rust_decimal_macros::dec!(5050)
        );
        let shown = q.prompt_text(&ri, Some(&p));
        assert!(
            shown.contains("$5050") && shown.contains("2024"),
            "the prompt quotes the year's figure: {shown}"
        );
    }

    // ── Identity, not position ───────────────────────────────────────────────────────────────────

    /// ★★★ **DELETE ROW 0 AND ROW 1'S DILIGENCE SURVIVES**, re-asserted through the REAL gates now
    /// that they exist. The `answer_log` key is the row's salted SSN hash, so nothing moves.
    #[test]
    fn deleting_row_zero_leaves_row_ones_gate_records_intact() {
        use crate::tax::provenance::{
            dependent_ssn_hash, record_answer, retire_dependent_identity, AnswerKey, AnswerState,
        };
        let mut ri = at_the_ctc_edge(2024);
        ri.header.dependents.push(Dependent {
            name: "Second Kid".into(),
            ssn: "000-00-2222".into(),
            relationship: "Son".into(),
            ..ri.header.dependents[0].clone()
        });
        for row in 0..2 {
            let ssn = ri.header.dependents[row].ssn.clone();
            for q in DEPENDENT_GATES {
                if !q.live(&ri, row) {
                    continue;
                }
                let prompt = q.prompt_text(&ri, None).into_owned();
                record_answer(
                    &mut ri,
                    AnswerKey::DependentGate {
                        ssn_hash: dependent_ssn_hash(&ssn),
                        gate: q.gate,
                    },
                    &prompt,
                    date!(2025 - 03 - 01),
                    AnswerState::Given,
                );
            }
        }
        let one = dependent_ssn_hash("000-00-2222");
        let before = ri
            .answer_log
            .keys()
            .filter(|k| matches!(k, AnswerKey::DependentGate { ssn_hash, .. } if *ssn_hash == one))
            .count();
        assert!(before > 0, "row 1 has records, or this asserts nothing");

        let zero_ssn = ri.header.dependents[0].ssn.clone();
        retire_dependent_identity(&mut ri, &zero_ssn);
        ri.header.dependents.remove(0);

        let after = ri
            .answer_log
            .keys()
            .filter(|k| matches!(k, AnswerKey::DependentGate { ssn_hash, .. } if *ssn_hash == one))
            .count();
        assert_eq!(
            after, before,
            "row 1's records are keyed by IDENTITY — deleting row 0 moves none of them"
        );
        let gone = dependent_ssn_hash(&zero_ssn);
        assert!(
            !ri.answer_log.keys().any(
                |k| matches!(k, AnswerKey::DependentGate { ssn_hash, .. } if *ssn_hash == gone)
            ),
            "and row 0's own records are gone, not orphaned onto the survivor"
        );
    }

    // ── The IDENTITY rules (seam review I-2) ─────────────────────────────────────────────────────

    /// ★★★ **A DEPENDENT ROW WITH A BLANK SSN REFUSES BEFORE ANY GATE, AND NAMES THE ROW.**
    ///
    /// `provenance.rs` recorded the question against this task — *"a row with a BLANK `ssn` has no
    /// identity, so every blank row shares one bucket … Recorded here so T7 decides it rather than
    /// meets it"* — and this is the decision. The gates on such a row cannot be answered truthfully
    /// at all: their records would be keyed by `dependent_ssn_hash("")`, which names nobody.
    #[test]
    fn a_dependent_row_with_no_ssn_refuses_before_any_gate_and_names_the_row() {
        let mut ri = at_the_ctc_edge(2024);
        ri.header.dependents[0].ssn = String::new();
        assert_eq!(
            screened(&ri),
            Some(RefuseReason::DependentIdentityUnanswered { row: 0 }),
            "the identity is demanded BEFORE the flowchart, not after it"
        );
        let detail = screened_detail(&ri);
        assert!(
            detail.contains("row 1 (Kid Example)"),
            "the refusal names the row the filer must fix: {detail}"
        );
        // ★ And it really is FIRST: blanking a gate as well must not change which rule speaks.
        ri.header.dependents[0].lived_with_you_over_half_year = None;
        assert_eq!(
            screened(&ri),
            Some(RefuseReason::DependentIdentityUnanswered { row: 0 }),
            "an identity-less row is not asked its §152 questions"
        );
    }

    /// ★★★ **TWO ROWS, ONE SSN — REFUSED, AND AT `income import` TOO.**
    ///
    /// A VALUE rule, not an unanswered one: nothing the filer can ANSWER separates two rows that
    /// share the key their answers are filed under. `income import` is the path that creates them,
    /// so that is where the exit is — the same line `ScreenTier` already draws for the census's own
    /// value rules.
    #[test]
    fn two_dependent_rows_with_the_same_ssn_refuse_at_commit_and_at_import() {
        use crate::tax::return_refuse::screen_param_free;
        let mut ri = at_the_ctc_edge(2024);
        let twin = ri.header.dependents[0].clone();
        ri.header.dependents.push(Dependent {
            name: "Twin Example".into(),
            // The SAME person's digits, punctuated differently — `dependent_ssn_hash` normalises
            // punctuation, so this is ONE identity and the screen must judge it the same way.
            ssn: twin.ssn.chars().filter(char::is_ascii_digit).collect(),
            ..twin
        });
        assert_eq!(
            screened(&ri),
            Some(RefuseReason::DependentSsnDuplicated { rows: (0, 1) }),
            "one SSN is one person: two rows under it would file one row's answers as the other's"
        );
        assert_eq!(
            screen_param_free(&ri).map(|r| r.reason),
            Some(RefuseReason::DependentSsnDuplicated { rows: (0, 1) }),
            "`income import` is the path that CREATES the rows — refusing there is the real exit"
        );
        let detail = screened_detail(&ri);
        for who in ["row 1 (Kid Example)", "row 2 (Twin Example)"] {
            assert!(
                detail.contains(who),
                "the refusal names both rows: {detail}"
            );
        }
    }

    /// ★★★ **`retire_dependent_identity` CANNOT CROSS-DELETE, because the state in which it would
    ///     is itself refused.**
    ///
    /// The mechanism is real and unchanged: `remove` on a row deletes every `answer_log` key under
    /// that row's `ssn_hash`, so two rows sharing a hash would have one row's removal take the
    /// other's diligence with it. What closes it is the rule above — no stored return may hold two
    /// rows with one key — so this asserts BOTH halves: the cross-deletion happens on the degenerate
    /// state, and the degenerate state cannot be stored.
    #[test]
    fn retiring_one_identity_cannot_take_anothers_records_because_a_shared_key_is_refused() {
        use crate::tax::provenance::{
            dependent_ssn_hash, record_answer, retire_dependent_identity, AnswerKey, AnswerState,
        };
        use crate::tax::return_refuse::screen_param_free;
        let mut ri = at_the_ctc_edge(2024);
        let twin = ri.header.dependents[0].clone();
        ri.header.dependents.push(Dependent {
            name: "Twin Example".into(),
            ..twin.clone()
        });
        // One record per row, written under each row's own key — which is the SAME key.
        for gate in [DependentGate::QcRelationship, DependentGate::Married] {
            let words = entry(gate).prompt_text(&ri, None).into_owned();
            record_answer(
                &mut ri,
                AnswerKey::DependentGate {
                    ssn_hash: dependent_ssn_hash(&twin.ssn),
                    gate,
                },
                &words,
                date!(2026 - 02 - 03),
                AnswerState::Given,
            );
        }
        // THE MECHANISM: retiring row 1 takes row 0's records too — two records for one `remove`.
        let mut degenerate = ri.clone();
        assert_eq!(
            retire_dependent_identity(&mut degenerate, &twin.ssn),
            2,
            "the premise: one identity, so one removal clears BOTH rows' diligence"
        );
        // THE CLOSURE: that return can never be stored, on either tier.
        assert_eq!(
            screen_param_free(&ri).map(|r| r.reason),
            Some(RefuseReason::DependentSsnDuplicated { rows: (0, 1) }),
            "…and no committed or imported return may hold two rows under one key"
        );
    }

    /// ★★★ **THE ROW-(5)(a) HELP CARRIES THE EXCEPTION, and R10.3 reaches the gates.** An answer
    /// recorded under one wording does not stand under another — the gates' prompts now resolve
    /// through `current_prompt`, so the re-ask rule covers them like every other key.
    #[test]
    fn a_gate_answered_under_earlier_words_is_refused_as_unanswered() {
        use crate::tax::provenance::{dependent_ssn_hash, record_answer, AnswerKey, AnswerState};
        let mut ri = at_the_ctc_edge(2024);
        let hash = dependent_ssn_hash(&ri.header.dependents[0].ssn);
        record_answer(
            &mut ri,
            AnswerKey::DependentGate {
                ssn_hash: hash,
                gate: DependentGate::LivedWithYouOverHalfYear,
            },
            "Did this person live with you for more than half of the year?", // the OLD words
            date!(2025 - 03 - 01),
            AnswerState::Given,
        );
        assert_eq!(
            screened(&ri),
            Some(RefuseReason::DependentGateUnanswered {
                row: 0,
                gate: DependentGate::LivedWithYouOverHalfYear
            }),
            "an answer given under earlier words does not stand under later ones"
        );
    }
}
