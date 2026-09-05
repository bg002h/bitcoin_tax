//! ★ P9 — the FORM QUESTION REGISTRY (`SPEC_form_questions.md` §3.1).
//!
//! The one place that knows the set of yes/no DECLARATIONS a return must answer. `screen_inputs`,
//! `income answer`, and `ReturnHeader::build` all DERIVE from this list, so no liveness predicate is
//! written twice — which is the whole point: the answered-ness invariant was the last load-bearing
//! invariant held by convention instead of construction (see [`super::return_inputs`]'s doc and D-8).

use crate::conventions::Usd;
use crate::tax::return_inputs::ReturnInputs;
use crate::tax::return_refuse::RefuseReason;
use crate::tax::types::FilingStatus;
use time::Date;

/// ★★★ **§G-15 — does this answer survive into the NEXT tax year?**
///
/// Answers are stored per tax year and **nothing carries forward silently** — correct, because a
/// prior year's "no" is not testimony for this year. But re-asking *everything* every year is waste
/// where the subject cannot change, and at the question counts the field census implies that waste
/// compounds annually.
///
/// ★★ **A prior-year answer must NEVER silently satisfy this year's provenance.** That is the
/// answered-ness invariant crossing a year boundary — software answering for the filer, one year
/// removed. The lawful shape is a **confirmation**, not a carry: *"Last year you said no. Still true
/// for 2025?"* is a NEW answer, given this year, bearing this year's date.
///
/// ★ **Defaults toward [`Durability::PerYear`]** — the fail-closed direction is to re-ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// The subject can differ year to year (foreign accounts, blindness, an election). Re-ask
    /// **blank**: for a one-keystroke answer, showing the prior buys nothing but anchoring.
    PerYear,
    /// The subject cannot change once known — a birth date. The prior MAY be displayed, but it still
    /// requires the same explicit keystroke as a fresh ask: never Enter-to-accept, never pre-filled.
    /// (Here a forced retype invites typos, and for a DOB a typo is the worse failure.)
    Durable,
}

/// A DECLARATION (§2, class A) — the filer ASSERTS it under §6065's jurat, so there is NO lawful default
/// and an unanswered one must REFUSE.
///
/// ONE entry per question, owning the prompt, the refusal, the refusal DETAIL, the liveness scope, and the
/// accessors. `screen_inputs`, `income answer`, and `ReturnHeader::build` DERIVE from this list.
pub struct FormQuestion {
    pub id: QuestionId,
    /// The prompt, phrased as the FORM phrases it (the words the filer can check against their paperwork).
    pub prompt: &'static str,
    /// The `RefuseReason` for an unanswered (`None`) live question.
    pub unanswered: RefuseReason,
    /// ★ The FULL refusal detail (r1 I-1). NOT derived from `prompt`: the shipped texts carry the statutory
    /// cite and the REMEDY (`run btctax income answer`) — doctrine requires the exit ("a refusal with no
    /// exit is just a brick with better prose"). A prompt-derived text would drop both.
    pub unanswered_detail: &'static str,
    /// ★ THE liveness predicate — the ONLY copy in the codebase.
    pub live: fn(&ReturnInputs) -> bool,
    /// Read the current answer.
    pub get: fn(&ReturnInputs) -> Option<bool>,
    /// Write an answer. Called only on a LIVE question (so, e.g., the mortgage setter may assume a
    /// `schedule_a` exists — its liveness requires one).
    pub set: fn(&mut ReturnInputs, bool),
    /// ★ §G-15 — whether this answer survives into the next tax year. See [`Durability`].
    pub durability: Durability,
    /// ★ The answer that requires **no adjustment and forgoes no benefit** — the "nothing to see here"
    /// reply. Most declarations are neutral at `false` ("no, I have no foreign trust"), but not all:
    /// the mortgage box is neutral at `true` (all of the loan bought/built/improved the home, so
    /// Schedule A line 8a stays full), and all three Form 6251 declarations are neutral at `true` (the
    /// dwelling IS AMT-qualified; the AMT carryover IS the same; the AMT depreciation IS the same).
    ///
    /// Declared per question rather than inferred, because polarity used to live as a hard-coded
    /// `matches!` in `testonly.rs` — knowledge a new question could silently get wrong.
    pub neutral: bool,
}

/// The identity of each registry question. `ALL` is the anchor the completeness test iterates; a new
/// variant is a compile error in that test until it is listed (§3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionId {
    DependentTaxpayer,
    DependentSpouse,
    MfsSpouseItemizes,
    ForeignAccounts,
    ForeignTrust,
    /// §2.4 — whether a Form 8889 trigger fired (renamed from the old "do you hold an HSA?").
    HsaActivity,
    /// §2.5 — the 1040 header dual-status-alien box.
    DualStatusAlien,
    /// §2.7 — the Schedule A line-8 mixed-use-mortgage box.
    MortgageAllUsedToBuyBuildImprove,
    /// **Form 6251 line 3** — is the mortgaged dwelling AMT-qualified? (i6251 p.8.)
    AmtQualifiedDwelling,
    /// **Form 6251 line 2k** — does the AMT capital-loss carryover equal the regular one?
    AmtCarryoverSameAsRegular,
    /// **Form 6251 line 2l** — is the depreciation inside the Schedule C expense total the same for the
    /// AMT as for the regular tax?
    AmtDepreciationSameAsRegular,
    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I** — did the filer exclude income under §911/931/933?
    HasIncomeExclusion,
    /// **§G-22 / B11** — the scope attestation: income this tool never asked about.
    OtherOutOfScopeIncome,
    /// **§163(h)(3)(B) / Schedule A line 8a** — is the combined home-acquisition debt inside every
    /// i1040sca *"Limits on home mortgage interest"* ceiling? ★ APPENDED AT THE END: `decl_tristate!`
    /// couples to this array's INDEX, so a mid-array insert silently repoints every later question.
    MortgageWithinDebtLimit,
    /// **Schedule D line 20 / Schedule A line 9** — is the filer filing Form 4952? ★ APPENDED AT THE
    /// END, for the `decl_tristate!` array-index reason above.
    FilingForm4952,
    /// **Capital Loss Carryover Worksheet header / §1212(b)** — does the carryover-in include a loss
    /// that was the SPOUSE'S, from a joint year now being filed separately? ★ APPENDED AT THE END,
    /// for the `decl_tristate!` array-index reason above.
    CarryoverIncludesSpousesJointLoss,
    /// **Capital Loss Carryover Worksheet header / §108(b)(2)(G)** — did the filer exclude canceled
    /// debt from income, requiring attribute reduction? ★ APPENDED AT THE END, same reason.
    ExcludedCanceledDebt,
}

impl QuestionId {
    pub const ALL: &'static [QuestionId] = &[
        QuestionId::DependentTaxpayer,
        QuestionId::DependentSpouse,
        QuestionId::MfsSpouseItemizes,
        QuestionId::ForeignAccounts,
        QuestionId::ForeignTrust,
        QuestionId::HsaActivity,
        QuestionId::DualStatusAlien,
        QuestionId::MortgageAllUsedToBuyBuildImprove,
        QuestionId::AmtQualifiedDwelling,
        QuestionId::AmtCarryoverSameAsRegular,
        QuestionId::AmtDepreciationSameAsRegular,
        QuestionId::HasIncomeExclusion,
        QuestionId::OtherOutOfScopeIncome,
        QuestionId::MortgageWithinDebtLimit,
        QuestionId::FilingForm4952,
        QuestionId::CarryoverIncludesSpousesJointLoss,
        QuestionId::ExcludedCanceledDebt,
    ];
}

/// ★★ §G-20 — do this return's SPOUSE §63(f) boxes count at all? MFJ always; **MFS only when all
/// three of i1040gi's conditions are affirmatively answered in the claiming direction.**
///
/// The ONE definition, shared by `AgedBlindBoxes::for_return` (which decides the deduction) and by the
/// liveness of `SpouseDiedDuringYear` / `DodSpouse` (which decide whether the questions are even
/// asked). Two copies would drift into asking a question whose answer nothing reads, or — worse —
/// counting a box whose carve-out was never posed.
pub fn spouse_63f_boxes_count(ri: &ReturnInputs) -> bool {
    ri.header.spouse.is_some() && spouse_63f_status_permits(ri)
}

/// Does the FILING STATUS (plus, on MFS, the three i1040gi conditions) permit a spouse §63(f) box —
/// **ignoring whether a spouse record exists**?
///
/// ★★★ r3 I-1 — the two predicates are split because their consumers ask different questions, and
/// collapsing them broke a case. [`spouse_63f_boxes_count`] decides the **deduction**, so it needs a
/// spouse record: no record, no date of birth, no box. The §63(f) **advisories** are about boxes that
/// were FORGONE, and an absent MFJ spouse record is *itself* one of the ways to forgo one — so they
/// must fire precisely where there is nothing to count. Gating them on `spouse_63f_boxes_count` made
/// the advisory silent in the case it exists to report (`mfj_with_no_spouse_record_still_advises_the_
/// aged_box_p5_m2`, which caught it).
///
/// Everything except the record test lives here, so the two can never disagree about the *status*
/// half — which is the coupling §G-20 was about.
pub fn spouse_63f_status_permits(ri: &ReturnInputs) -> bool {
    match ri.filing_status {
        FilingStatus::Mfj => true,
        FilingStatus::Mfs => {
            ri.header.spouse_had_no_income == Some(true)
                && ri.header.spouse_not_filing_a_return == Some(true)
                && ri.header.can_be_claimed_as_dependent_spouse == Some(false)
        }
        _ => false,
    }
}

/// Is `id`'s question LIVE on this return? The single accessor for a liveness predicate outside the
/// registry loop.
///
/// ★ Exists so a VALUE-refusal (`Some(false)` ⇒ refuse) can share the exact predicate its UNANSWERED
/// half uses, instead of re-deriving it. An ungated value-refusal is an exit-less brick: a stale
/// adverse answer left over from a Schedule A that no longer carries mortgage interest, or a Schedule C
/// whose expenses dropped to $0, would refuse a return whose add-back is structurally $0 — with no way
/// for the filer to clear it, because the question is no longer asked. Re-deriving the predicate at the
/// refusal site is exactly the duplication `FormQuestion::live` was introduced to end (§3.1).
pub fn question_is_live(id: QuestionId, ri: &ReturnInputs) -> bool {
    FORM_QUESTIONS
        .iter()
        .find(|q| q.id == id)
        .is_some_and(|q| (q.live)(ri))
}

/// ★★ Does this return bring a capital-loss carryforward IN at all?
///
/// The ONE predicate behind all three carryforward-conditioned declarations — Form 6251 line 2k, the
/// Capital Loss Carryover Worksheet's joint-return sourcing rule, and its §108(b)(2)(G) canceled-debt
/// condition. Factored out so a fourth cannot be written with a fourth copy of the same test.
///
/// ★★★ **It is deliberately NOT widened with a taxable-income term.** The tempting shape — "only ask
/// when the year is at the floor, since that is where the carryover matters" — is the understatement
/// direction: a positive-taxable-income year with a mis-attributed joint carryover still deducts a
/// loss that is not the filer's, and still rolls it forward. `widening-an-exemption-is-never-the-safe-
/// edit`: enumerate the YES-condition (a carryforward exists) and let every other case fail closed.
pub fn carryforward_in_present(ri: &ReturnInputs) -> bool {
    let cf = ri.capital_loss_carryforward_in;
    cf.short > Usd::ZERO || cf.long > Usd::ZERO
}

/// Whether an AMT capital-loss-carryover twin could exist — Form 6251 line 2k's liveness.
fn amt_carryover_question_live(ri: &ReturnInputs) -> bool {
    carryforward_in_present(ri)
}

/// Whether a Form 6251 line 2l depreciation adjustment could be hiding inside the Schedule C expense
/// total — line 2l's liveness.
///
/// ★ An INPUT predicate, like [`amt_carryover_question_live`] and unlike anything compute-dependent:
/// `schedule_c` present with a nonzero expense total. We cannot ask a narrower question, and that is
/// precisely the point — [`ScheduleCInputs::expenses`] is a flat total, so btctax can never see whether
/// Schedule C Part II line 13 ("Depreciation and section 179 expense deduction") is $0 or $200,000. Any
/// filer with business expenses at all must therefore affirm. See [`RefuseReason::
/// AmtDepreciationDeclarationUnanswered`] for why the alternative — assuming $0 — is unsound.
///
/// ★ **The prompt enumerates a narrow YES-list and defaults to NO — deliberately, structurally.**
///
/// This wording took THREE tries, and the first two failed in opposite directions. That history is the
/// design rationale, so it is recorded rather than tidied away:
///   1. v1 listed the 200%-DB trigger broadly and would have refused every filer who owns equipment.
///      Fail-closed, so merely bricking.
///   2. v2 "fixed" that by granting exemptions — and asserted an UNCONDITIONAL straight-line exemption.
///      i6251 qualifies every "isn't refigured" bullet with **placed in service after 1998**, and its
///      must-refigure list carries "Tangible property placed in service after 1986 and before 1999"
///      with no method qualifier. A filer with a 1990s building would have answered yes truthfully and
///      omitted a required add-back — an UNDERSTATEMENT.
///   3. v3 narrowed the pre-1999 hole but still said "no adjustment applies to post-1998 property
///      depreciated ... 150% declining balance", dropping the instructions' parenthetical **"(other
///      than section 1250 property)"**. Post-1998 §1250 property not depreciated straight-line — 15-
///      and 20-year land improvements: paving, fencing, site utilities — is on the MUST-refigure list.
///      Another understatement, and note the qualifier was present in THIS doc comment and lost on the
///      way into the prompt: a paraphrase of a paraphrase.
///
/// **So the structure, not the wording, is the fix.** Enumerating NO-triggers with a broad "otherwise
/// yes" fallback makes every omission an understatement. Enumerating YES-conditions with a "otherwise
/// no" fallback makes every omission an over-refusal, which is fail-closed and recoverable. The prompt
/// now does the latter and says "if you are unsure, answer NO" outright. Adding a missing exemption
/// later is a safe edit; widening the fallback is not.
///
/// Each permitted YES is individually grounded in i6251 (2024) p.5:
///   - no depreciation claimed AND none capitalized ⇒ line 2l is $0 by arithmetic. The capitalization
///     rider is not pedantry: i6251 says "you must refigure depreciation for the AMT, **including
///     depreciation allocable to inventory costs**", and a filer who capitalized rather than deducted
///     would otherwise read "claimed no depreciation" as true;
///   - "Any part of the cost of any property for which you elected to take a section 179 expense
///     deduction" (a fully-§179'd asset leaves no remaining basis to refigure);
///   - "Qualified property that is or was eligible for a special depreciation allowance …" plus "It
///     isn't subject to an AMT adjustment for depreciation if it was placed in service after 2015".
///     ★ A gloss reading "(most equipment bought since 2016)" was REMOVED from this condition: it is
///     not sourced to i6251, and bonus-INELIGIBLE 200%-DB equipment exists (used property acquired
///     before 9/28/2017; related-party and carryover-basis acquisitions), which the MUST list catches.
///     The operative words "qualified for bonus depreciation" already exclude it — but reassuring
///     prose next to a gate is what produced all three earlier defects, so the reassurance goes;
///   - the four straight-line bullets, which between them cover post-1998 §1250 and non-§1250 property.
///     ★ "for the regular tax" is load-bearing and is stated in the prompt: i6251 always writes
///     "depreciated **for the regular tax** using the straight line method", because post-1998 §1250
///     property is straight-line *for the AMT* while possibly 150%-DB for the regular tax. Dropping
///     those three words would re-admit the land improvements that v3 got wrong;
///   - "Property for which you elected to use the alternative depreciation system (ADS) of section
///     168(g) for the regular tax" (no date limit).
///
/// Passive, at-risk, partnership-basis and farm-shelter depreciation route to lines 2m/2n/3 instead, so
/// the prompt's silence on them is correct.
fn amt_depreciation_question_live(ri: &ReturnInputs) -> bool {
    ri.schedule_c
        .as_ref()
        .is_some_and(|c| c.expenses > Usd::ZERO)
}

/// Whether Schedule A carries mortgage interest — the mixed-use question's liveness. Deliberately an
/// INPUT predicate (`schedule_a.is_some() ∧ mortgage_interest_1098 > 0`), NOT "Schedule A files" (which is
/// compute-dependent and would brick the standard-deduction-wins filer — §2.7, r3 I-2).
fn mortgage_question_live(ri: &ReturnInputs) -> bool {
    ri.schedule_a
        .as_ref()
        .is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)
}

/// ★ THE REGISTRY. Eleven declarations; the liveness lifted from the shipped refusals EXCEPT the two P9
/// corrections — `DependentSpouse` widened to `Mfj || spouse.is_some()` (= P8a I1) and the two foreign
/// questions made live ALWAYS (= §2.9, the circular-liveness bug in shipped code).
pub const FORM_QUESTIONS: &[FormQuestion] = &[
    FormQuestion {
        id: QuestionId::DependentTaxpayer,
        prompt: "Can someone claim YOU as a dependent on their return?",
        unanswered: RefuseReason::DependentStatusUnanswered,
        unanswered_detail:
            "every return must state whether someone can claim YOU as a dependent (it selects the \
             §63(c)(5) standard-deduction floor and is a checkbox on the 1040) — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.header.can_be_claimed_as_dependent_taxpayer,
        set: |ri, v| ri.header.can_be_claimed_as_dependent_taxpayer = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DependentSpouse,
        prompt: "Can someone claim YOUR SPOUSE as a dependent on their return?",
        unanswered: RefuseReason::DependentSpouseStatusUnanswered,
        unanswered_detail:
            "this return has (or is) a joint filing, so it must state whether someone can claim YOUR \
             SPOUSE as a dependent (it is a checkbox on the 1040) — run `btctax income answer`",
        // ★ = P8a I1: MFJ makes the box live even when the spouse `Person` is absent; a stale spouse on a
        // non-MFJ return is a recorded over-ask (§3.1), never an under-ask.
        live: |ri| ri.filing_status == FilingStatus::Mfj || ri.header.spouse.is_some(),
        get: |ri| ri.header.can_be_claimed_as_dependent_spouse,
        set: |ri, v| ri.header.can_be_claimed_as_dependent_spouse = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::MfsSpouseItemizes,
        prompt: "Does your spouse ITEMIZE deductions on their separate return? (§63(c)(6) forces your \
                 choice to match theirs)",
        unanswered: RefuseReason::MfsSpouseItemizeUnknown,
        unanswered_detail:
            "a married-filing-separately return must state whether the spouse itemizes (§63(c)(6)) — \
             run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Mfs,
        get: |ri| ri.mfs_spouse_itemizes,
        set: |ri, v| ri.mfs_spouse_itemizes = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ForeignAccounts,
        prompt: "Schedule B line 7a: did you have a financial interest in, or signature authority over, \
                 a FOREIGN financial account?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 7a (a foreign financial account) must be answered on every return — \
             it is the FBAR/FinCEN disclosure, and its own answer is what decides whether Schedule B files — \
             run `btctax income answer`",
        // ★ = §2.9: live ALWAYS. It CANNOT be scoped by `schedule_b_files`, because that predicate reads
        // this very answer — the circular liveness that silently omitted Schedule B in shipped code.
        live: |_ri| true,
        get: |ri| ri.foreign_accounts,
        set: |ri, v| ri.foreign_accounts = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ForeignTrust,
        prompt: "Schedule B line 8: did you receive a distribution from — or were you the grantor of, or \
                 transferor to — a FOREIGN TRUST?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 8 (a foreign trust) must be answered on every return — a foreign \
             trust independently requires Part III, so it cannot be scoped by whether Schedule B otherwise \
             files — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.foreign_trust,
        set: |ri, v| ri.foreign_trust = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaActivity,
        prompt: "In this tax year, did ANY of these happen with a health savings account? — (a) anyone \
                 (you, your employer, or anyone else on your behalf) put money into one for you; (b) you \
                 took money out of one; (c) you inherited one; or (d) you stopped being HSA-eligible after \
                 using the last-month rule or an IRA-to-HSA funding distribution in a prior year.",
        unanswered: RefuseReason::HsaActivityUnanswered,
        unanswered_detail:
            "a return must state whether a Form 8889 trigger fired for a health savings account (a \
             contribution by anyone, a distribution, a testing-period inclusion, or an inheritance) — an \
             unasked distribution omits gross income and a 20% additional tax (§223(f)) — run `btctax \
             income answer`",
        live: |_ri| true,
        get: |ri| ri.sch1.hsa_activity,
        set: |ri, v| ri.sch1.hsa_activity = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DualStatusAlien,
        prompt: "Were you a DUAL-STATUS ALIEN this year (a nonresident alien for part of the year and a \
                 resident for the rest)?",
        unanswered: RefuseReason::DualStatusAlienUnanswered,
        unanswered_detail:
            "a return must state whether you were a dual-status alien — the 1040 header prints that box, \
             and §63(c)(6)(B) zeroes a nonresident alien's standard deduction — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.dual_status_alien,
        set: |ri, v| ri.dual_status_alien = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::MortgageAllUsedToBuyBuildImprove,
        prompt: "Did you use ALL of your home-mortgage loan(s) to buy, build, or improve that home? \
                 (Schedule A line 8: if not, the box is checked.)",
        unanswered: RefuseReason::MixedUseMortgageUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so it must state whether the loan(s) were all used \
             to buy, build, or improve the home (§163(h)(3)(F) — Schedule A line 8) — run `btctax income \
             answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_all_used_to_buy_build_improve)
        },
        // Live requires `schedule_a.is_some()`, so the `if let` always fires when this is called on a live
        // question; the guard is defensive (a caller that set on a non-live question is a no-op, not a panic).
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_all_used_to_buy_build_improve = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // §2.7: "yes, all of it" keeps Schedule A line 8a full
    },
    FormQuestion {
        id: QuestionId::AmtQualifiedDwelling,
        prompt: "Is the home your Form 1098 mortgage interest relates to a principal residence, or a \
                 house, apartment, condominium or mobile home NOT used on a transient basis? (Form 6251 \
                 line 3 — a houseboat or recreational vehicle is NOT an AMT-qualified dwelling.)",
        unanswered: RefuseReason::AmtQualifiedDwellingUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so Form 6251 line 3 must know whether the dwelling \
             is AMT-qualified — interest on a dwelling that is not a principal residence or an \
             AMT-qualified dwelling is ADDED BACK for the alternative minimum tax (i6251, Line 3). \
             Guessing would understate the tax — run `btctax income answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_dwelling_is_amt_qualified)
        },
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_dwelling_is_amt_qualified = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, AMT-qualified" ⇒ Form 6251 line 3 adds nothing back
    },
    FormQuestion {
        id: QuestionId::AmtCarryoverSameAsRegular,
        prompt: "Is your capital-loss carryover for the alternative minimum tax the SAME as your \
                 regular-tax carryover? (Form 6251 line 2k — answer no if you have ever tracked a \
                 separate AMT basis or AMT capital-loss carryforward.)",
        unanswered: RefuseReason::AmtCarryoverDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, so Form 6251 line 2k must know whether the \
             AMT carryover differs from the regular-tax one — btctax tracks only the regular figure, and \
             a divergent AMT twin is an ADD-BACK. Guessing would understate the tax — run \
             `btctax income answer`",
        live: amt_carryover_question_live,
        get: |ri| ri.amt_carryover_same_as_regular,
        set: |ri, v| ri.amt_carryover_same_as_regular = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, the same" ⇒ Form 6251 line 2k adds nothing back
    },
    FormQuestion {
        id: QuestionId::AmtDepreciationSameAsRegular,
        prompt: "Is the depreciation included in your Schedule C expenses the SAME for the alternative \
                 minimum tax as for the regular tax? (Form 6251 line 2l.) Answer YES only if one of \
                 these is true of EVERY depreciable asset in that total: you claimed no depreciation at \
                 all, and none was capitalized into inventory; or you deducted its whole cost under \
                 section 179; or it was placed in service after 2015 AND qualified for bonus \
                 depreciation; or it is depreciated STRAIGHT-LINE FOR THE REGULAR TAX and was placed in \
                 service after 1998; or you elected \
                 ADS for it. Answer NO if any asset falls outside that list — in particular anything \
                 placed in service before 1999, 200% declining-balance property from 1999-2015, and \
                 land improvements or other section 1250 property depreciated 150% declining balance. \
                 If you are unsure, answer NO: that refuses the return rather than risking an \
                 understated tax.",
        unanswered: RefuseReason::AmtDepreciationDeclarationUnanswered,
        unanswered_detail:
            "this return carries Schedule C expenses, and btctax accepts that as a FLAT TOTAL — it never \
             sees Part II line 13 ('Depreciation and section 179 expense deduction'), so it cannot tell \
             whether a Form 6251 line 2l adjustment is hiding inside it. A divergent AMT amount is an \
             ADD-BACK, so it is never guessed away: the prompt lists the conditions that permit a yes, \
             and if none clearly applies the answer is NO. Guessing yes would understate the tax — run \
             `btctax income answer`",
        live: amt_depreciation_question_live,
        get: |ri| ri.amt_depreciation_same_as_regular,
        set: |ri, v| ri.amt_depreciation_same_as_regular = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, the same" ⇒ Form 6251 line 2l adds nothing back
    },
    FormQuestion {
        id: QuestionId::HasIncomeExclusion,
        prompt: "Did you exclude any income from gross income under section 911 (foreign earned \
                 income or housing), section 931 (American Samoa) or section 933 (Puerto Rico)? \
                 (Schedule 1-A Part I / the Schedule A state-and-local-tax worksheet — these \
                 exclusions are ADDED BACK to figure modified AGI.)",
        unanswered: RefuseReason::IncomeExclusionUnanswered,
        unanswered_detail:
            "modified AGI is adjusted gross income increased by any §911/931/933 exclusion, and it \
             drives the §164(b) SALT phase-down and all four Schedule 1-A deductions. Treating an \
             unasked exclusion as zero UNDERSTATES modified AGI, which RAISES those deductions and \
             understates the tax — run `btctax income answer`",
        // ★★ §G-15 — YEAR-SCOPED at last. This shipped ALWAYS LIVE because `live` received only
        // `&ReturnInputs`, which carried no tax year, so it could not be scoped to "years that
        // compute modified AGI" — and TY2024 filers were therefore asked a TY2025 question. That was
        // defensible only because a bespoke neutrality proof existed for it: `Some(false)` ⇒
        // modified AGI = AGI, exactly what TY2024's `FlatCap` assumes and never reads, so no TY2024
        // figure could move.
        //
        // ★ The proof does NOT generalise, which is why the workaround had to go rather than be
        // repeated: Schedule 1-A Part IV asks about a deduction that did not exist in TY2024, so a
        // "no" there answers a question with no TY2024 legal meaning — testimony about nothing.
        //
        // ★ `ReturnInputs::tax_year` is stamped from the storage row key on read, so this predicate
        // reads a year that is true by construction. A year-0 (never stored, never stated) fixture
        // is NOT ≥ 2025, so it is not live — which is the fail-closed direction: an unstated year
        // must not conjure a TY2025 question.
        live: |ri| ri.tax_year >= 2025,
        get: |ri| ri.has_income_exclusion,
        set: |ri, v| ri.has_income_exclusion = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false, // "no exclusions" is the AMT/MAGI-neutral answer, but it is still an ANSWER
    },
    // ── §G-22 / B11: the SCOPE ATTESTATION. ─────────────────────────────────────────────────────────
    FormQuestion {
        id: QuestionId::OtherOutOfScopeIncome,
        prompt: "In this tax year, did ANY of these happen? (a) You received income other than what \
                 you have entered here — a PENSION, ANNUITY or IRA DISTRIBUTION (Form 1099-R), \
                 SOCIAL SECURITY or railroad retirement benefits (Form SSA-1099 or RRB-1099), rent \
                 or royalties, a farm, a partnership, S corporation, estate or trust (any Schedule \
                 K-1), unreported tips, gambling winnings, alimony, a business this tool did not \
                 capture, or anything else it never asked about. (b) You \
                 EXERCISED AN INCENTIVE STOCK OPTION (ISO) and still held the stock at the end of the \
                 year — you would have a Form 3921. (c) You had any other item this tool never asked \
                 about that changes your ALTERNATIVE MINIMUM TAX — depletion, a tax-shelter farm \
                 activity, a passive activity, or research and experimental costs.",
        unanswered: RefuseReason::OtherIncomeUnanswered,
        unanswered_detail:
            "btctax asks about HSA activity, dual-status alien status and foreign accounts, and a \
             `yes` to any of them stops the return — but it never asked whether you had rental, \
             royalty, farm or K-1 income, so silence LOOKED like `none` and a return could file with \
             §61 income left off it. It now also asks whether you EXERCISED AN INCENTIVE STOCK OPTION \
             (Form 6251 line 2i), which is not income at all for the regular tax and so was invisible \
             to the income half of this question. Silence is not testimony that there is none: answer \
             it — run `btctax income answer`",
        // ★★★ LIMB (b) IS AN ISO EXERCISE, AND IT IS NOT INCOME — which is exactly why it had to be
        //     added here rather than left to the income half. i6251, first sentence of the line-2i
        //     instruction: *"For the regular tax, no income is recognized when an incentive stock
        //     option (ISO), as defined in section 422(b), is exercised. However, this rule doesn't
        //     apply for the AMT."* So a truthful filer with a $180,000 ISO adjustment answered the OLD
        //     prompt — *"did you RECEIVE any income…"* — with a truthful **No**, and the gate stayed
        //     shut.
        //
        //     ★★★ AND THE GAP HID ITS OWN DETECTION. `Form6251::must_attach()` is `line7 > line10`,
        //     and the missing 2i add-back is exactly what would have pushed line 7 past line 10. So
        //     the return did not merely print a wrong 2i — it never tripped `AmtScreenTriggered` at
        //     all and filed clean, with no Form 6251 and no AMT, on a return signed under §6065.
        //     Understating, invisible to both oracles, and invisible to every value test.
        //
        //     ★★ Limb (c) is the same argument generalised: `form6251.rs` models lines 2, 2a and 2b
        //     only, so every other Part I add-back (2c–2t) is silently zero. Naming the four that a
        //     btctax filer could plausibly have — rather than asking "any AMT item?" — follows the
        //     enumerate-the-YES-conditions rule: a filer cannot answer `no` to a category they were
        //     never shown.
        //
        //     ★ WIDENING a mandatory question's YES-conditions is the SAFE direction of edit, which is
        //     why it is done here rather than by adding a second question. The unsafe direction —
        //     widening an EXEMPTION — is what `widening-an-exemption-is-never-the-safe-edit` names.
        //
        // ★★★ ALWAYS LIVE, and deliberately so. Every other class-(A) declaration is scoped to the
        // years or shapes that read it; this one is read by NOTHING — it exists precisely because
        // there is no field for the income it asks about. A liveness predicate here could only be
        // "was the filer likely to have some?", which is the guess the question exists to refuse to
        // make. ★ It is also stable across years in a way per-schedule questions are not: the
        // out-of-scope SET moves every tax year, the union does not.
        live: |_| true,
        get: |ri| ri.other_out_of_scope_income,
        set: |ri, v| ri.other_out_of_scope_income = Some(v),
        durability: Durability::PerYear,
        // ★★ NOT neutral. "No other income" is an affirmative statement about the filer's year that no
        // default may make on their behalf — that is the whole finding. `false` here would let the
        // answer be assumed, restoring the silence this question exists to break.
        neutral: false,
    },
    // ── §G-9: the §63(f) death carve-out. Two entries, because i1040gi states the rule twice — once
    // under "Death of a taxpayer" and once under "Death of spouse" — and each is a separate fact.
    //

    // ★★★ §163(h)(3)(B) — THE ACQUISITION-DEBT CEILING. Index 13; APPENDED AT THE END for the
    //     array-index reason stated on `QuestionId::MortgageWithinDebtLimit`.
    //
    // ★★ THE PROMPT ENUMERATES THE YES-CONDITIONS AND FALLS BACK TO NO, which is the whole shape of
    //    it (`widening-an-exemption-is-never-the-safe-edit`). i1040sca states FOUR limits under
    //    *"Limits on home mortgage interest"*; one of them — the mixed-use limit — is already its own
    //    question (index 7), so the three AMOUNT limits are listed here individually. A vaguer
    //    "were you within the limits?" would be laundered into three answers the filer never gave,
    //    and each omission would fail OPEN into a full deduction the statute caps.
    //
    // ★ Same liveness as the other two mortgage questions — `mortgage_question_live`, the EXISTING
    //   predicate, unchanged. A Schedule A that reports 1098 interest is exactly the return on which
    //   line 8a can be wrong, and the limit is not derivable from anything btctax holds: it collects
    //   the INTEREST, never the balance, the origination date, or the home's fair market value.
    FormQuestion {
        id: QuestionId::MortgageWithinDebtLimit,
        prompt: "Schedule A line 8a — were you inside EVERY home-mortgage debt limit this year? \
                 Answer YES only if all of these are true of your home mortgages counted together: \
                 (a) qualifying debt taken out AFTER December 15, 2017 never exceeded $750,000 \
                 ($375,000 if married filing separately); (b) qualifying debt taken out ON OR BEFORE \
                 December 15, 2017 never exceeded $1,000,000 ($500,000 if married filing separately) \
                 — and if you have both kinds, the $750,000 limit is REDUCED by the amount of the \
                 older debt; and (c) the total of all your mortgages was never more than the home's \
                 fair market value. Answer NO if any one of them was exceeded, and answer NO if you \
                 are unsure: a NO refuses the return and sends you to Pub. 936's Deductible Home \
                 Mortgage Interest Worksheet, rather than risking an understated tax.",
        unanswered: RefuseReason::MortgageDebtLimitUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so it must state whether your combined home \
             acquisition debt stayed inside the §163(h)(3)(B) limits (i1040sca, \"Limits on home \
             mortgage interest\": $750,000/$375,000 for qualifying debt taken out after December 15, \
             2017; $1,000,000/$500,000 for debt taken out on or before it; and the home's fair market \
             value). btctax collects the INTEREST, never the balance — so left unasked it deducts the \
             whole Form 1098 amount, which for a filer over the limit UNDERSTATES the tax. Neither \
             oracle can catch that: both take line 8a as an INPUT (§G-9). Run `btctax income answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_within_debt_limit)
        },
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_within_debt_limit = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one. Debt balances move.
        durability: Durability::PerYear,
        neutral: true, // "yes, inside every limit" ⇒ Schedule A line 8a stays the full 1098 amount
    },
    // ★★★ SCHEDULE D LINE 20 / SCHEDULE A LINE 9 — THE FORM 4952 DECLARATION. Index 14; appended at
    //     the END for the array-index reason above.
    //
    // ★★★ ALWAYS LIVE, and it has to be. Line 20 prints on every return whose Schedule D routes
    //     both-gains, and that routing comes from the LEDGER — which `live` (a `&ReturnInputs`
    //     predicate) cannot see. Scoping it to `schedule_a.is_some()` would under-ask exactly the
    //     population the plan measured: the $0-income, standard-deduction crypto household whose
    //     Schedule D still prints the same sworn "Yes". A question that vanishes for the filer it was
    //     written for is the shape §G-9 exists to kill.
    //
    // ★★ THE PROMPT ENUMERATES WHEN FORM 4952 IS REQUIRED and falls back to YES — which is the
    //    fail-closed direction here, because YES refuses and refusals are recoverable. The list is
    //    i4952's own exception, negated: you may answer NO only if every one of its three conditions
    //    holds. A filer who is unsure answers YES and gets a refusal they can undo, rather than a
    //    filed return whose line 20 they never saw.
    FormQuestion {
        id: QuestionId::FilingForm4952,
        prompt: "Are you filing Form 4952 (Investment Interest Expense Deduction)? Answer NO only \
                 if ALL THREE of these are true — they are Form 4952's own exception: (a) your \
                 investment interest expense is not more than your investment income from interest \
                 and ordinary dividends minus any qualified dividends; (b) you have no other \
                 deductible investment expenses; and (c) you have no disallowed investment interest \
                 expense carried over from last year. Answer YES if you borrowed to invest and any \
                 of those fails, and answer YES if you are unsure: a YES refuses the return rather \
                 than filing a Schedule D line 20 that swears you are not filing a form you are. \
                 (Schedule D line 20 asks \"Are lines 18 and 19 both zero or blank and you are not \
                 filing Form 4952?\"; Schedule A line 9 is \"Investment interest. Attach Form 4952 \
                 if required.\")",
        unanswered: RefuseReason::Form4952DeclarationUnanswered,
        unanswered_detail:
            "Schedule D line 20 asks whether lines 18 and 19 are both zero or blank AND you are not \
             filing Form 4952 — and its answer decides which worksheet computes your tax. btctax \
             checked \"Yes\" on every both-gains return without ever asking you, which is sworn \
             testimony it invented. It will not do that: answer it and the return files (a \"No\" \
             routes to the Schedule D Tax Worksheet, which btctax does not fill). The same answer \
             governs Schedule A line 9, investment interest — run `btctax income answer`",
        // ★ See the block comment above: it CANNOT be scoped by whether Schedule D files, because
        //   that is a ledger fact and `live` receives only `ReturnInputs`.
        live: |_ri| true,
        get: |ri| ri.filing_form_4952,
        set: |ri, v| ri.filing_form_4952 = Some(v),
        // ★ §G-15 — PER-YEAR: whether you file Form 4952 is a fact about this tax year.
        durability: Durability::PerYear,
        // ★ NOT filing Form 4952 is the answer that needs no form btctax lacks: line 20 = Yes ⇒ the
        //   Qualified Dividends and Capital Gain Tax Worksheet, which btctax does compute.
        neutral: false,
    },
    // ★★★ THE CAPITAL LOSS CARRYOVER WORKSHEET'S TWO UNNUMBERED HEADER CONDITIONS. Indices 15 and 16;
    //     appended at the END for the `decl_tristate!` array-index reason above.
    //
    // ★★★ WHY THEY EXIST AT ALL. The worksheet header states two governing conditions in prose, above
    //     line 1 — and the conformance checker's completeness half reads only physical lines beginning
    //     `N.`, so both were STRUCTURALLY INVISIBLE to it and were dropped while it stayed green
    //     (`xtask::capital_loss_carryover_check::unnumbered_conditions_in_the_form` is the half that
    //     now sees them). `CLAUDE.md`: *"If the form asks something our input surface cannot answer,
    //     collect it. That is following instructions, not scope creep."*
    //
    // ★★★ AND WHY THEY BECAME LOAD-BEARING NOW. Before `--write-carryover` rolled the §1212(b)
    //     figure, a mis-attributed or unreduced carryover was at worst the filer's own bad input.
    //     After it, btctax re-emits that figure as its OWN `Computed` value on next year's Schedule D
    //     lines 6 and 14 — sworn under §6065. That is the one edit that turns a user error into a
    //     btctax assertion, so both questions fail CLOSED: `None` refuses, and so does `Some(true)`.
    FormQuestion {
        id: QuestionId::CarryoverIncludesSpousesJointLoss,
        prompt: "Does any part of your capital-loss carryover come from a JOINT return for a year \
                 you are now filing separately from, where the loss was your SPOUSE'S? (Capital Loss \
                 Carryover Worksheet header: \"If you and your spouse once filed a joint return and \
                 are filing separate returns for 2025, any capital loss carryover from the joint \
                 return can be deducted only on the return of the spouse who actually had the \
                 loss.\") Answer NO only if the whole carryover is your own loss — because you have \
                 never filed jointly, or because you are still filing jointly with the same spouse, \
                 or because every dollar of it was realized on property that was yours. Answer YES \
                 if any part of it was your spouse's, and answer YES if you are unsure: a YES \
                 refuses the return rather than deducting a loss that is not yours.",
        unanswered: RefuseReason::JointReturnCarryoverDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, and the Capital Loss Carryover \
             Worksheet's header says a carryover from a joint return \"can be deducted only on the \
             return of the spouse who actually had the loss\" (§1212(b)). btctax stores ONE \
             carryover per return and has no way to tell whose loss it was, so it cannot make that \
             split for you — and with `--write-carryover` it would re-emit the figure as its own \
             computed entry on next year's Schedule D lines 6 and 14. Run `btctax income answer`",
        live: carryforward_in_present,
        get: |ri| ri.carryover_includes_spouses_joint_loss,
        set: |ri, v| ri.carryover_includes_spouses_joint_loss = Some(v),
        // ★ §G-15 — PER-YEAR. Filing status changes; so does which spouse's loss is still running.
        durability: Durability::PerYear,
        // ★ NOT neutral at true: a YES is the ADVERSE answer here, and it refuses. `false` — "all of
        //   it is mine" — is the answer that needs no split btctax cannot perform.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ExcludedCanceledDebt,
        prompt: "Did you exclude cancelled or forgiven debt from your income this year — for \
                 example under the insolvency, bankruptcy, or qualified-principal-residence rules \
                 (Form 982)? (Capital Loss Carryover Worksheet header: \"If you excluded canceled \
                 debt from income in 2025, see Pub. 4681.\") Answer NO only if you excluded none. \
                 Answer YES if you filed or should have filed Form 982, and answer YES if you are \
                 unsure: a YES refuses the return rather than carrying forward a loss that \
                 §108(b)(2)(G) requires you to reduce.",
        unanswered: RefuseReason::ExcludedCanceledDebtDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, and the Capital Loss Carryover \
             Worksheet's header sends a filer who excluded canceled debt to Pub. 4681 — because \
             §108(b) then requires TAX ATTRIBUTE REDUCTION, and §108(b)(2)(G) puts capital loss \
             carryovers on that list. btctax models no part of §108(b), so a carryover it has not \
             been told to reduce is too large, and with `--write-carryover` it would persist that \
             figure as next year's computed input. Run `btctax income answer`",
        live: carryforward_in_present,
        get: |ri| ri.excluded_canceled_debt,
        set: |ri, v| ri.excluded_canceled_debt = Some(v),
        // ★ §G-15 — PER-YEAR: a debt exclusion is an event of one tax year.
        durability: Durability::PerYear,
        // ★ NOT neutral at true: a YES is the ADVERSE answer and refuses.
        neutral: false,
    },
];

/// The identity of each SKIPPABLE prompt (§2, class B) — the questions where silence is LAWFUL: a bare
/// Enter leaves the value `None`, forgoing a benefit whose burden to CLAIM is the filer's (New Colonial
/// Ice), and the matching advisory then fires (never in silence — the owner mandate).
///
/// ★ A SEPARATE identity space from [`QuestionId`] (spec §5.3 HARD RULE). A skippable is `None`-legal; a
/// [`FormQuestion`] declaration is not. Merging the two registries would brick `screen_inputs` — it would
/// refuse a lawfully-unanswered skippable — so the two lists must never be one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkippableId {
    /// ★ §63(f) BLINDNESS (taxpayer). Always live; `None` forgoes the addition and fires the advisory.
    BlindTaxpayer,
    /// ★ §63(f) BLINDNESS (spouse) — live only with a spouse `Person` (a `set_bool` on an absent spouse is
    /// silently discarded, so the prompt is gated to match).
    BlindSpouse,
    /// ★ §164(b)(5) sales-tax election — live only with a `schedule_a` (nowhere to write it otherwise).
    SalesTaxElection,
    /// §63(f) aged addition (taxpayer). A mandatory DOB prompt would force the filer to INVENT a birthday,
    /// and an invented-old one understates tax — so `None` must stay reachable.
    DobTaxpayer,
    /// §63(f) aged addition (spouse) — live only with a spouse `Person` (its `set_date` twin gate).
    DobSpouse,
    /// §G-9: the DATE OF DEATH (taxpayer). Class (B) — skipping it leaves `is_aged` unable to show the
    /// taxpayer reached 65, so the addition is FORGONE, never granted. Live only once
    /// [`Self::TaxpayerDiedDuringYear`] is answered `Some(true)`: nobody else has a date to give.
    DodTaxpayer,
    /// §G-9: the DATE OF DEATH (spouse). Live only once [`Self::SpouseDiedDuringYear`] is `Some(true)`.
    DodSpouse,
    /// ★★ **1040 line 12a / §63(f) — the death carve-out (`FOLLOWUPS.md` §G-9), taxpayer.** i1040gi:
    /// *"Death of a taxpayer … If a taxpayer was born before January 2, 1961, but died in 2025 before
    /// reaching age 65, then the taxpayer doesn't qualify."*
    ///
    /// ★★★ **It was a class-(A) declaration that REFUSED, and it was `live: |_| true` — so it blocked
    /// EVERY return btctax could compute.** That is the single biggest usability cost in the registry,
    /// and it bought nothing: [`crate::tax::return_1040::is_aged`]'s `(None, None)` arm already returns
    /// `false`, so silence FORGOES the addition. The refusal was redundant with a fail-safe sitting
    /// directly beneath it. Class (B) by the sharp test — *does the silence ASSERT, or FORGO?* It
    /// forgoes, and [`crate::tax::advisories::Advisory::AgedBoxForfeitedDeathUnanswered`] says so, but
    /// only when a qualifying date of birth is on file and the skip therefore actually costs money.
    TaxpayerDiedDuringYear,
    /// **1040 line 12a / §63(f) — the death carve-out, spouse.** i1040gi: *"If your spouse was born
    /// before January 2, 1960, but died in 2024 before reaching age 65, don't check the box that says
    /// 'Spouse was born before January 2, 1960.'"* Same class-(B) reasoning as
    /// [`Self::TaxpayerDiedDuringYear`].
    ///
    /// ★ **Live exactly when the spouse's §63(f) boxes can count** — `spouse_63f_boxes_count`, not
    /// merely "a spouse record exists". On MFJ that is any spouse; on MFS it is a spouse meeting all
    /// three i1040gi conditions. Anywhere else the answer could not move a figure, and it was
    /// previously asked (and, before that, REFUSED) on returns where it was inert.
    SpouseDiedDuringYear,
    /// ★★ **Schedule B line 7a's unnumbered FBAR sub-question** — *"If 'Yes,' are you required to file
    /// FinCEN Form 114 … ?"* Live only when 7a is answered **Yes**: the form itself conditions it on 7a
    /// ("If 'Yes,'"), so a filer with no foreign account is never asked.
    ///
    /// ★★★ **It was briefly a class-(A) refusal, and that was WRONG.** Refusal is justified only when
    /// proceeding without the answer would produce a wrong number, put fabricated testimony on a signed
    /// return, or silently expose the filer to a penalty or a lost right. This box fails all three:
    /// **no figure on the return reads it** (grep `fbar_filing_required` — the printed chain writes the
    /// checkbox and nothing else), a blank is *no testimony* rather than false testimony, and the
    /// penalty the form's Caution warns of attaches to **not filing FinCEN Form 114** — a FinCEN
    /// obligation that exists whatever this box says — not to leaving the box blank. That exposure is
    /// already put in front of the filer by [`super::advisories::Advisory::FbarFinCen`], which fires on
    /// 7a = Yes alone.
    ///
    /// So silence is lawful and prints a genuine blank; [`super::advisories::Advisory::FbarSubQuestionNotAnswered`]
    /// quotes the form's Caution **verbatim** when it is skipped. btctax takes no position on the
    /// answer: FinCEN Notice 2020-2 leaves accounts holding ONLY virtual currency outside the FBAR
    /// requirement for now, that is under active reconsideration, and an account holding crypto PLUS
    /// fiat or securities may well be reportable.
    FbarFilingRequired,
    /// ★★ **Schedule C line I** — *"Did you make any payments in 2024 that would require you to file
    /// Form(s) 1099?"* A compliance declaration about the filer's own information-reporting.
    ///
    /// Class (B), not a refusal, by the refusal review's criterion applied honestly: no figure on the
    /// return reads it, a blank is no testimony, and — **unlike Schedule B's FBAR sub-question** —
    /// the form prints NO Caution beside it. That difference is what decided it. The §6721/§6722
    /// exposure IS real, so the skip is not silent:
    /// [`super::advisories::Advisory::ScheduleC1099NotAnswered`] names both sections.
    ScheduleC1099Required,
    /// **Schedule C line J** — *"If 'Yes,' did you or will you file required Form(s) 1099?"*
    ///
    /// ★★ Live only when line I is answered **Yes** — the form says *"If 'Yes,'"* — which makes this
    /// the registry's live example of a question whose liveness depends on another question's
    /// **NON-NEUTRAL** answer. That shape silently broke the registry's own property harness once: a
    /// loop answering every other live question at its neutral switched this class of question OFF, so
    /// it could never be exercised. The `is_none()` guards in `return_refuse.rs` were written for it
    /// and, until now, had no live case.
    ScheduleC1099Filed,
    /// ★★★ **Form 8283 Section B lines 5a / 5b / 5c**, asked as ONE return-level universal: *"did any
    /// of your donations have strings attached?"* A **Yes** to any of the three limbs shrinks or kills
    /// the §170 deduction (Reg §1.170A-7), and btctax deducts at full FMV — so a Yes means the number
    /// is WRONG. Asked here so silence never auto-refuses a filer who donated NOTHING; the refusal is scoped by
    /// `screen_absolute`, which has the computed §63(e) ELECTION that liveness cannot see, and is MANDATORY there:
    /// on a Section-B year, unanswered refuses and `Some(true)` refuses — only `Some(false)` proceeds.
    ///
    /// ★★ **This is what dissolved §G-21.** The three boxes are per-donation and the registry is
    /// return-shaped, which looked like it needed new per-row machinery. Asking the UNIVERSAL makes
    /// them return-shaped too: from *"none of my donations had any of these"* each box's answer follows
    /// for every donation. The prompt therefore enumerates all three limbs in the form's own words — a
    /// "No" to something vaguer would be laundered into three answers the filer never gave.
    DonationsHadRestrictions,
    /// **Form 8995-A Part I column (b)** — is the trade or business a specified service trade or
    /// business? (§G-28/B1b.)
    ScheduleCIsSstb,
    /// **Form 8995-A Part I column (e)** — is the filer a patron of an agricultural or horticultural
    /// cooperative? (§G-28/B1b.) Unlike every other question here this one decides **which form is
    /// filed**, at any level of income.
    ScheduleCIsCooperativePatron,
    /// ★★★ **§170(f)(8) — the CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT**, Schedule A lines 11/12's own
    /// *"If you made any gift of $250 or more, see instructions"*. Asked as one return-level
    /// universal, exactly like [`Self::DonationsHadRestrictions`]; the MANDATORY half lives in
    /// `screen_absolute`, which has the ledger AND the computed §63(e) itemize election.
    CharitableCwaObtained,
}

/// The value shape of a [`SkippableQuestion`] — a yes/no answer, or a calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkippableKind {
    YesNo,
    Date,
}

/// A SKIPPABLE prompt (§2.2, class B). The same fn-pointer shape as [`FormQuestion`], but silence is a
/// LAWFUL outcome here: a bare Enter leaves the value `None`. The accessors split by [`kind`](Self::kind):
/// the pair that does not apply to this question's `kind` returns `None` / is a no-op (the catch-all lifted
/// from the old `answer.rs::Skippable`). Each `set` is also a no-op when its target row is absent — which
/// is exactly why `live` gates the spouse/Schedule-A prompts, so the prompt scope tracks the WRITE scope.
pub struct SkippableQuestion {
    /// ★ §G-15 — whether this answer survives into the next tax year. See [`Durability`].
    ///
    /// ★★ This is where `Durable` actually occurs: a **date of birth cannot change**. Everything
    /// else here can — blindness, a §164(b)(5) election, and a date of DEATH (gated on
    /// `…DiedDuringYear`, itself a per-year declaration).
    pub durability: Durability,
    pub id: SkippableId,
    /// The prompt, phrased as the FORM phrases it (the words the filer can check against their paperwork).
    pub prompt: &'static str,
    /// What skipping forgoes — the advisory framing, for a UI that shows help beside the prompt.
    pub help: &'static str,
    /// Whether this prompt reads a yes/no or a date — the answer loop branches on it.
    pub kind: SkippableKind,
    /// ★ THE liveness predicate — the ONLY copy, lifted from the old `answer.rs::live_questions` gates.
    pub live: fn(&ReturnInputs) -> bool,
    /// The yes/no on file (`None` for the `Date` kinds).
    pub get_bool: fn(&ReturnInputs) -> Option<bool>,
    /// Record a yes/no (a no-op for the `Date` kinds, or when the target row is absent).
    pub set_bool: fn(&mut ReturnInputs, bool),
    /// The date on file (`None` for the `YesNo` kinds).
    pub get_date: fn(&ReturnInputs) -> Option<Date>,
    /// Record a date (a no-op for the `YesNo` kinds, or when the target row is absent).
    pub set_date: fn(&mut ReturnInputs, Date),
}

/// ★ THE SKIPPABLE REGISTRY. Thirteen prompts — SEPARATE from [`FORM_QUESTIONS`] (spec §5.3). The
/// liveness gates and prompts are lifted verbatim from the old `answer.rs::Skippable`; the `income answer`
/// flow and the form engine both DERIVE their skippable prompts from this one list.
///
/// ★★ **Two species, and the second is why class (B) is not simply "forgone benefit".**
///
/// - **Benefit claims** (*New Colonial Ice*: the burden to claim is the filer's, so forgoing is
///   lawful) — blindness ×2, the SALT election, the DOBs, the dates of death, and the §G-9 death
///   gates, whose silence forgoes the §63(f) age-65 addition.
/// - **Compliance boxes NO FIGURE READS** — [`SkippableId::FbarFilingRequired`] and the Schedule C
///   pair [`SkippableId::ScheduleC1099Required`] / [`SkippableId::ScheduleC1099Filed`]. Their silence
///   neither asserts nor forgoes, because nothing on the return depends on them. They are class (B)
///   for a different reason from everything else here, and each has an advisory naming the exposure
///   the blank does NOT remove (FinCEN 114; §6721/§6722).
///
/// **Class (B) is the set of questions whose silence is LAWFUL, not only the set that costs money.**
pub const SKIPPABLE_QUESTIONS: &[SkippableQuestion] = &[
    SkippableQuestion {
        id: SkippableId::BlindTaxpayer,
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Are YOU legally blind? (§63(f) additional deduction)",
        help: "§63(f): legal blindness adds an extra standard-deduction amount. Skipping leaves it \
               unclaimed — lawful, since the burden to claim is yours — and the forgone-benefit advisory fires.",
        kind: SkippableKind::YesNo,
        live: |_ri| true,
        get_bool: |ri| ri.header.taxpayer.blind,
        set_bool: |ri, v| ri.header.taxpayer.blind = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::BlindSpouse,
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Is YOUR SPOUSE legally blind? (§63(f) additional deduction)",
        help: "§63(f): the spouse's legal blindness adds an extra standard-deduction amount. Skipping \
               leaves it unclaimed and the forgone-benefit advisory fires.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.header.spouse.is_some(),
        get_bool: |ri| ri.header.spouse.as_ref().and_then(|s| s.blind),
        set_bool: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.blind = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::SalesTaxElection,
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Deduct general SALES taxes instead of state/local income taxes? (§164(b)(5))",
        help: "§164(b)(5): elect to deduct general sales taxes instead of state and local income taxes. \
               Skipping keeps income taxes on the return; the election is advised when a Schedule A exists.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.schedule_a.is_some(),
        get_bool: |ri| ri.schedule_a.as_ref().and_then(|a| a.salt_use_sales_tax),
        set_bool: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.salt_use_sales_tax = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::DobTaxpayer,
        // ★★ §G-15 — DURABLE: a date of birth cannot change. The prior MAY be shown, but it still
        // takes the same explicit keystroke as a fresh ask — a forced retype invites a typo, and
        // for a DOB a typo is the worse failure.
        durability: Durability::Durable,
        prompt: "YOUR date of birth",
        help: "§63(f): your date of birth establishes the age-65 additional standard deduction. Skipping \
               leaves it unclaimed — a mandatory prompt would force you to invent a birthday, so silence stays reachable.",
        kind: SkippableKind::Date,
        live: |_ri| true,
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_date: |ri| ri.header.taxpayer.date_of_birth,
        set_date: |ri, v| ri.header.taxpayer.date_of_birth = Some(v),
    },
    SkippableQuestion {
        id: SkippableId::DobSpouse,
        // ★★ §G-15 — DURABLE: a date of birth cannot change. The prior MAY be shown, but it still
        // takes the same explicit keystroke as a fresh ask — a forced retype invites a typo, and
        // for a DOB a typo is the worse failure.
        durability: Durability::Durable,
        prompt: "YOUR SPOUSE's date of birth",
        help: "§63(f): the spouse's date of birth establishes the age-65 additional standard deduction. \
               Skipping leaves it unclaimed.",
        kind: SkippableKind::Date,
        live: |ri| ri.header.spouse.is_some(),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_date: |ri| ri.header.spouse.as_ref().and_then(|s| s.date_of_birth),
        set_date: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.date_of_birth = Some(v);
            }
        },
    },
    SkippableQuestion {
        id: SkippableId::DodTaxpayer,
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "YOUR date of death",
        help: "§63(f) / §G-9: a taxpayer who died during the year before reaching age 65 does not get \
               the age-65 addition. The date decides it — a person reaches 65 on the DAY BEFORE their \
               65th birthday. Skipping leaves the addition unclaimed (it is never granted on an \
               unknown date).",
        kind: SkippableKind::Date,
        live: |ri| ri.header.taxpayer_died_during_year == Some(true),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_date: |ri| ri.header.taxpayer.date_of_death,
        set_date: |ri, v| ri.header.taxpayer.date_of_death = Some(v),
    },
    SkippableQuestion {
        id: SkippableId::DodSpouse,
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "YOUR SPOUSE's date of death",
        help: "§63(f) / §G-9: a spouse who died during the year before reaching age 65 does not get the \
               age-65 addition. A person reaches 65 on the DAY BEFORE their 65th birthday. Skipping \
               leaves the addition unclaimed.",
        kind: SkippableKind::Date,
        // Three conditions: MFJ (the only status whose spouse box `AgedBlindBoxes::for_return` counts),
        // a spouse `Person` to write the date onto, AND the gate saying they died. The MFJ term matches
        // `SkippableId::SpouseDiedDuringYear`'s own liveness — a date whose gate is never asked would be
        // unreachable, and asking for a date that cannot move a figure is the waste this pass removed.
        live: |ri| spouse_63f_boxes_count(ri) && ri.header.spouse_died_during_year == Some(true),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_date: |ri| ri.header.spouse.as_ref().and_then(|s| s.date_of_death),
        set_date: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.date_of_death = Some(v);
            }
        },
    },
    SkippableQuestion {
        id: SkippableId::FbarFilingRequired,
        // ★ §G-15 — PER-YEAR: whether an FBAR is required turns on the year's account balances.
        durability: Durability::PerYear,
        prompt: "Schedule B line 7a (sub-question): you said you had a foreign financial account \u{2014} \
                 are you REQUIRED to file FinCEN Form 114 (the FBAR) to report that financial interest or \
                 signature authority?",
        help: "Schedule B's own Caution: \"If required, failure to file FinCEN Form 114 may result in \
               substantial penalties. Additionally, you may be required to file Form 8938, Statement of \
               Specified Foreign Financial Assets.\" That penalty attaches to NOT FILING FinCEN Form 114 \
               \u{2014} an obligation independent of this box \u{2014} not to leaving the box blank, so \
               skipping is lawful and prints a true blank. btctax takes no position on the answer: FinCEN \
               Notice 2020-2 leaves crypto-only accounts outside the requirement for now, that is under \
               active reconsideration, and an account holding crypto PLUS fiat or securities may well be \
               reportable.",
        kind: SkippableKind::YesNo,
        // ★ The FORM conditions this on 7a \u{2014} "If 'Yes,' are you required to file\u{2026}".
        live: |ri| ri.foreign_accounts == Some(true),
        get_bool: |ri| ri.fbar_filing_required,
        set_bool: |ri, v| ri.fbar_filing_required = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★ THE DEATH PAIR (§G-9). Downgraded from class-(A) refusing declarations, where
    // `TaxpayerDiedDuringYear` was `live: |_| true` and therefore blocked EVERY return. See the
    // `SkippableId` docs for why the refusal was redundant: `is_aged`'s `(None, None)` arm already
    // forgoes the addition, so silence has always failed in the safe direction.
    //
    // ★ WHY A GATE PLUS A DATE, rather than asking "did they die before reaching 65?" directly: the
    // day-before-the-birthday convention is exactly the sort of arithmetic a filer gets wrong at the
    // boundary, and we can do it exactly. The gate carries answered-ness, `date_of_death` carries the
    // fact, and `is_aged` applies the convention. Both halves are now class (B).
    SkippableQuestion {
        id: SkippableId::TaxpayerDiedDuringYear,
        // ★ §G-15 — PER-YEAR by definition: the question names a tax year.
        durability: Durability::PerYear,
        prompt: "Did YOU (the taxpayer named on this return) die during the tax year? (A final return \
                 is filed by a personal representative or surviving spouse. 1040 line 12a: a taxpayer \
                 who died before reaching age 65 does not get the age-65 addition to the standard \
                 deduction, however early in the year they were born.)",
        help: "§63(f) / §G-9. Skipping is lawful and costs nothing unless a date of birth on file would \
               otherwise have qualified you for the age-65 addition — in that case the addition is \
               FORGONE (never granted on an unresolved death carve-out), your tax is OVERSTATED, and \
               `Advisory::AgedBoxForfeitedDeathUnanswered` says so with the amount.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS LIVE, and that is now harmless. It cannot be scoped to "a DOB old enough to matter"
        // because `live` receives no tax year (the constraint `HasIncomeExclusion` documents), and
        // scoping it to "a DOB is on file" would make the question vanish and reappear as the filer
        // edits — the never-asked-then-silently-relevant shape §G-9 exists to kill. Always-live is only
        // a problem when the question REFUSES; as a skippable it is one Enter.
        live: |_ri| true,
        get_bool: |ri| ri.header.taxpayer_died_during_year,
        set_bool: |ri, v| ri.header.taxpayer_died_during_year = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::SpouseDiedDuringYear,
        durability: Durability::PerYear,
        prompt: "Did YOUR SPOUSE die during the tax year? (You may still file jointly for the year of \
                 death. 1040 line 12a: a spouse who died before reaching age 65 does not get the \
                 age-65 addition to the standard deduction, however early in the year they were born.)",
        help: "§63(f) / §G-9, spouse. Skipping forgoes the spouse's age-65 box if a qualifying date of \
               birth is on file; the advisory names the amount.",
        kind: SkippableKind::YesNo,
        // ★ Asked exactly when a spouse box can COUNT — the same predicate `AgedBlindBoxes::for_return`
        // uses, so the question and the figure can never disagree. Previously `spouse.is_some()`, which
        // asked (and refused) on returns where nothing could read the reply.
        live: spouse_63f_boxes_count,
        get_bool: |ri| ri.header.spouse_died_during_year,
        set_bool: |ri, v| ri.header.spouse_died_during_year = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },

    // ★★ SCHEDULE C's Form-1099 COMPLIANCE PAIR (lines I and J). Class (B) by the refusal review's
    // criterion: no figure reads them, a blank is no testimony, and the form prints NO Caution beside
    // them — unlike Schedule B's FBAR sub-question, which is the distinction that decided this. The
    // §6721/§6722 exposure is real, so the skip fires an advisory naming both sections.
    SkippableQuestion {
        id: SkippableId::ScheduleC1099Required,
        durability: Durability::PerYear,
        prompt: "Schedule C line I: did you make any payments this year that would require you to file \
                 Form(s) 1099? (For example, $600 or more to a contractor or service provider for your \
                 business. See the Schedule C instructions.)",
        help: "§6721/§6722 penalise failing to file a required information return and failing to \
               furnish the payee's copy. Skipping is lawful — no figure on your return reads this box, \
               and btctax will never answer it for you — but the box goes out BLANK and the exposure \
               is yours either way.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.payments_requiring_1099),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.payments_requiring_1099 = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::ScheduleC1099Filed,
        durability: Durability::PerYear,
        prompt: "Schedule C line J: did you, or will you, file those required Form(s) 1099?",
        help: "Asked only because you answered line I \"Yes\" — the form itself says \"If 'Yes,'\". \
               Skipping leaves the box blank; §6721/§6722 still apply.",
        kind: SkippableKind::YesNo,
        // ★ THE FORM CONDITIONS IT ON LINE I — "If 'Yes,'". A Schedule C AND a Yes on I.
        live: |ri| {
            ri.schedule_c
                .as_ref()
                .is_some_and(|c| c.payments_requiring_1099 == Some(true))
        },
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.will_file_required_1099),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.will_file_required_1099 = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },

    // ★★★ Form 8283 Section B lines 5a/5b/5c, asked as ONE return-level universal (§G-21).
    SkippableQuestion {
        id: SkippableId::DonationsHadRestrictions,
        durability: Durability::PerYear,
        // ★★ THE PROMPT ENUMERATES ALL THREE LIMBS, in the form's own words. A "No" to something
        // vaguer ("any strings?") would be laundered into three specific answers the filer never gave
        // — the widening-an-exemption failure. Enumerate the YES-conditions; anything not listed is
        // then something the filer has NOT denied, so every omission fails closed.
        prompt: "Did ANY property you donated this year have strings attached? Answer YES if any of \
                 these is true of ANY donation: (a) there is a restriction, temporary or permanent, on \
                 the charity's right to USE or DISPOSE of it; (b) you gave anyone other than the \
                 charity a right to its INCOME, to POSSESS it, to VOTE it, or to ACQUIRE it; or (c) \
                 there is a restriction limiting it to a PARTICULAR USE. (Form 8283 lines 5a/5b/5c.)",
        help: "Skipping is harmless if you donated nothing, or nothing over $5,000 — Form 8283 asks \
               these only in Section B. On a year that DOES file a Section B, it is MANDATORY: a \
               \"Yes\" to any limb reduces or denies the §170 deduction (Reg §1.170A-7 — a gift with \
               retained rights is not a gift of the whole thing), and btctax deducts at full fair \
               market value, so it refuses rather than file a number it knows is too large.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS OFFERED: `live` receives only `ReturnInputs`, and the donations are in the LEDGER.
        //   Silence is lawful HERE so a filer who donated nothing is never blocked; the mandatory half
        //   lives in `screen_absolute`, which has the ledger AND the itemize election, and can tell.
        live: |_ri| true,
        get_bool: |ri| ri.donations_had_restrictions,
        set_bool: |ri, v| ri.donations_had_restrictions = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §G-28/B1b — Form 8995-A Part I column (b), the SSTB checkbox.
    //
    // ★ APPENDED AT THE END, like every other entry here. `skippable_tristate!` in the input-form
    //   registry couples to this array's INDEX, so inserting mid-array silently repoints every later
    //   question — placing this before `DonationsHadRestrictions` in draft did exactly that.
    SkippableQuestion {
        id: SkippableId::ScheduleCIsSstb,
        durability: Durability::PerYear,
        prompt: "Is your business a SPECIFIED SERVICE trade or business? Answer YES if its principal \
                 asset is the reputation or skill of its owners or employees, or if it is in health, \
                 law, accounting, actuarial science, performing arts, consulting, athletics, financial \
                 services, brokerage services, or investing and investment management. (Form 8995-A, \
                 Part I, column (b).)",
        help: "Skipping is harmless below the §199A(e)(2) threshold — the simplified Form 8995 does \
               not ask, because the answer changes nothing there. ABOVE it, it is MANDATORY: past the \
               phase-in range an SSTB's qualified business income is EXCLUDED ENTIRELY \
               (§199A(d)(3)), so an unasked \"no\" would hand you a deduction the statute denies and \
               understate your tax. btctax cannot infer it from your business description — it is a \
               checkbox on the form because only you can answer it.",
        kind: SkippableKind::YesNo,
        // ★★ ALWAYS OFFERED, MANDATORY ONLY WHERE IT MATTERS — the `DonationsHadRestrictions` shape,
        //    and for the same reason. `live` sees only `ReturnInputs`, which cannot know taxable
        //    income; making this a live DECLARATION refused EVERY Schedule C return at every income
        //    level, including the great majority for whom the answer is irrelevant. The mandatory half
        //    lives in `screen_absolute`, which knows the threshold.
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.is_sstb),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.is_sstb = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §G-28/B1b — Form 8995-A Part I column (e), the PATRON checkbox. Appended at the END, for the
    //     array-index reason above.
    SkippableQuestion {
        id: SkippableId::ScheduleCIsCooperativePatron,
        durability: Durability::PerYear,
        prompt: "Are you a patron of an agricultural or horticultural cooperative? Answer YES if a \
                 cooperative paid you patronage dividends, per-unit retain allocations, or passed \
                 through a section 199A(g) deduction for this business. (Form 8995-A, Part I, \
                 column (e).)",
        help: "Skipping is harmless if this business has no qualified business income. Where it DOES, \
               the answer decides which form is filed at ANY income: Form 8995-A's own header says to \
               use it if your taxable income is above the threshold \"or you're a patron of an \
               agricultural or horticultural cooperative\", and Form 8995 says the same in reverse. A \
               \"Yes\" REFUSES — the patron reduction comes from Schedule D (Form 8995-A), which \
               btctax does not fill, and filing without it would OVERSTATE your deduction.",
        kind: SkippableKind::YesNo,
        // ★ Offered wherever there is a trade or business to be a patron through; the mandatory half
        //   is in `screen_absolute`, which knows whether a §199A form is actually being printed.
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.is_cooperative_patron),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.is_cooperative_patron = Some(v);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §170(f)(8) — THE CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT. Index 15; appended at the END for
    //     the array-index reason stated above.
    //
    // ★★ THE PROMPT MUST BE ANSWERABLE **YES** BY A FILER WITH NO ≥$250 GIFT. i1040sca: *"In figuring
    //    whether a gift is $250 or more, don't combine separate donations"* — so a filer who gave $25
    //    a week has no ≥$250 gift at all and answers yes vacuously. That matters because btctax holds
    //    non-crypto gifts as one amount per entry, which may itself be a roll-up of small gifts: the
    //    gate can over-ASK, but the question must never over-CLAIM.
    SkippableQuestion {
        id: SkippableId::CharitableCwaObtained,
        durability: Durability::PerYear,
        // ★★★ THE WORDING COVERS THE DEFERRED CLAIM TOO (final whole-branch review, finding 2).
        //
        // This used to be scoped to gifts "you are deducting this year". Phase 2's R3 fold widened
        // the GATE to §170(b)-ceiling-deferred claims but left these words alone — so a wholly
        // deferred filer deducts nothing this year, reads the question literally, answers YES with
        // perfect honesty while holding no acknowledgment, and the gate passes. The cure then dies at
        // filing anyway. Mechanism and text were adjudicated in different rounds and nobody re-read
        // the words after the population widened.
        prompt: "For EVERY charitable gift of $250 or more that this return DEDUCTS — this year, or \
                 in a later year because it exceeded its §170(b) percentage-of-income ceiling and is \
                 carrying forward — do you \
                 already hold — or will you obtain before you file — a CONTEMPORANEOUS WRITTEN \
                 ACKNOWLEDGMENT from the charity showing (1) the amount of money and a description \
                 (but not the value) of any property donated, and (2) whether the organization gave \
                 you any goods or services in return, with a description and estimate of their value \
                 if it did? (Schedule A lines 11 and 12: \"If you made any gift of $250 or more, see \
                 instructions.\" In figuring whether a gift is $250 or more, don't combine separate \
                 donations — so answer YES if you made no single gift that large. Don't attach the \
                 acknowledgment to your return; keep it for your records. \
                 ★ ANSWER FOR THE CARRYOVER TOO: a gift held back by the ceiling is DEFERRED, not \
                 denied, and §170(f)(8)(C)'s deadline still runs from THIS return — so \"I am \
                 deducting nothing this year\" is not a reason to answer yes.)",
        help: "Skipping is harmless if you claim no charitable deduction AND none of this year's \
               gifts is carrying forward, or if you made no single gift of \
               $250 or more. Where a deduction IS claimed — now or later — it is MANDATORY: \
               §170(f)(8)(A) says \"No \
               deduction shall be allowed … for any contribution of $250 or more unless the taxpayer \
               substantiates the contribution by a contemporaneous written acknowledgment\" — a \
               precondition of the deduction itself, not a recordkeeping nicety. \
               ★ THE DEADLINE IS WHY THIS IS ASKED NOW: §170(f)(8)(C) makes an acknowledgment \
               contemporaneous only if you get it \"by the date you file your return or the due date \
               (including extensions) for filing your return, whichever is earlier\". You can still \
               get one — right up until you file. Once you file without it, the cure is gone.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS OFFERED — the `DonationsHadRestrictions` shape, and for the same two reasons:
        //   `live` sees only `ReturnInputs`, so it can see neither the LEDGER (where the crypto
        //   donations are) nor the computed §63(e) itemize election. Making this a live class-(A)
        //   DECLARATION would refuse every return at every income level, including the standard-
        //   deduction filers the adjudication says must never be asked. The mandatory half is in
        //   `screen_absolute`, which can tell.
        live: |_ri| true,
        get_bool: |ri| ri.charitable_cwa_obtained,
        set_bool: |ri, v| ri.charitable_cwa_obtained = Some(v),
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    }
];

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ THE COMPLETENESS ANCHOR (§3.5). Anchored to the ENUM, not to `FORM_QUESTIONS` — an anti-vacuity
    /// test that ITERATED the list would silently drop its own scenario when an entry was dropped (r1 I-4).
    /// The `match` is exhaustive, so a NEW `QuestionId` variant is a COMPILE ERROR until it is listed here —
    /// a human MUST edit this test, right next to the hardcoded `len() == 8` tripwires. The index round-trip
    /// (r2 M-3) then catches a MIS-ORDERED `ALL`. (Honest limit, IMPL r1 M-1: a human who adds the match arm
    /// but forgets the `ALL` element AND the count still slips through — the compiler forces the edit, not
    /// its correctness, exactly as §3.3 states.)
    #[test]
    fn every_question_id_is_in_all_in_order_and_has_exactly_one_entry() {
        for (i, id) in QuestionId::ALL.iter().enumerate() {
            let idx = match id {
                QuestionId::DependentTaxpayer => 0,
                QuestionId::DependentSpouse => 1,
                QuestionId::MfsSpouseItemizes => 2,
                QuestionId::ForeignAccounts => 3,
                QuestionId::ForeignTrust => 4,
                QuestionId::HsaActivity => 5,
                QuestionId::DualStatusAlien => 6,
                QuestionId::MortgageAllUsedToBuyBuildImprove => 7,
                QuestionId::AmtQualifiedDwelling => 8,
                QuestionId::AmtCarryoverSameAsRegular => 9,
                QuestionId::AmtDepreciationSameAsRegular => 10,
                QuestionId::HasIncomeExclusion => 11,
                QuestionId::OtherOutOfScopeIncome => 12,
                QuestionId::MortgageWithinDebtLimit => 13,
                QuestionId::FilingForm4952 => 14,
                QuestionId::CarryoverIncludesSpousesJointLoss => 15,
                QuestionId::ExcludedCanceledDebt => 16,
            };
            assert_eq!(idx, i, "QuestionId::ALL is out of order / missing {id:?}");
            assert_eq!(
                FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(),
                1,
                "exactly one FORM_QUESTIONS entry for {id:?}"
            );
        }
        assert_eq!(QuestionId::ALL.len(), 17, "there are 17 declarations");
        assert_eq!(FORM_QUESTIONS.len(), 17, "one entry per declaration");
    }

    /// ★★★ §G-6/ISO — THE OUT-OF-SCOPE QUESTION MUST NAME THE ISO EXERCISE, WHICH IS NOT INCOME.
    ///
    /// `form6251.rs` models Part I lines 2, 2a and 2b only; 2c–2t are absent, and line **2i** is the
    /// exercise of an incentive stock option — the dominant real AMT trigger post-TCJA, since the 2017
    /// Act removed the SALT and miscellaneous-deduction add-backs that used to drive individual AMT.
    ///
    /// ★★★ THE OLD PROMPT COULD NOT CATCH IT, and the reason is in i6251's own first sentence:
    /// *"For the regular tax, no income is recognized when an incentive stock option (ISO), as defined
    /// in section 422(b), is exercised. However, this rule doesn't apply for the AMT."* The question
    /// asked *"did you RECEIVE any income…"*, so a filer with a $180,000 ISO adjustment answered a
    /// truthful **No** and the gate stayed shut.
    ///
    /// ★★ AND THE GAP HID ITS OWN DETECTION: `must_attach()` is `line7 > line10`, and the missing 2i
    /// add-back is exactly what would have pushed line 7 past line 10 — so the return never tripped
    /// `AmtScreenTriggered`, filed clean with no Form 6251 and no AMT, and understated tax on a return
    /// signed under §6065. Invisible to both oracles and to every value-checking test.
    ///
    /// This pins the PROMPT, because the prompt is the whole mechanism: nothing computes from it.
    #[test]
    fn the_out_of_scope_question_names_the_iso_exercise_and_the_amt_items() {
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::OtherOutOfScopeIncome)
            .expect("the out-of-scope declaration exists");
        let p = q.prompt.to_ascii_lowercase();
        assert!(
            p.contains("incentive stock option") && p.contains("3921"),
            "the prompt must name the ISO exercise AND the form the filer already holds: {}",
            q.prompt
        );
        assert!(
            p.contains("alternative minimum tax"),
            "…and must name the AMT category, since lines 2c-2t are all silently zero: {}",
            q.prompt
        );
        // ★★★ **RETIREMENT INCOME, added 2026-09-04, and the reason is an UNDERSTATEMENT path.**
        //     This enumeration primes the filer: someone holding a 1099-R and an SSA-1099 who reads
        //     "rent or royalties, a farm, a partnership…" and finds nothing resembling their pension
        //     can answer a truthful-feeling **No** on the strength of the list, even though the
        //     trailing "or anything else it never asked about" formally covers it. §61 and §86 income
        //     then leaves the return silently. btctax models no line 4a-6b at all, so the ONLY thing
        //     standing between a retiree and an understated return is this sentence naming their
        //     forms. Name the FORM NUMBERS, not just the category — the filer is holding the paper.
        //     ★ Removing any of these must red: that is the whole guarantee, since nothing computes
        //     from this prompt. See design/ty2025/SPEC_retirement_income.md (OQ-1).
        for limb in [
            "pension",
            "annuity",
            "1099-r",
            "social security",
            "ssa-1099",
        ] {
            assert!(
                p.contains(limb),
                "the prompt must name retirement income and the form the filer already holds — \
                 `{limb}` is missing, and a retiree who answers No on the strength of this list \
                 files omitting §61/§86 income: {}",
                q.prompt
            );
        }
        // ★ The income limbs must SURVIVE the widening — this question's original job is unchanged.
        for limb in ["rent", "royalt", "k-1", "alimony", "gambling"] {
            assert!(
                p.contains(limb),
                "the income limbs must remain: {limb} missing"
            );
        }
        // ★★ And it must still be a MANDATORY class-(A) declaration that refuses unanswered. A prompt
        //    nobody has to answer would make all of the above decoration.
        assert!((q.live)(&ReturnInputs::default()), "always live");
        assert_eq!(q.unanswered, RefuseReason::OtherIncomeUnanswered);
        assert_eq!(
            (q.get)(&ReturnInputs::default()),
            None,
            "and `Default` must not answer it"
        );
    }

    #[test]
    fn skippable_registry_is_separate_and_has_five_entries_with_correct_liveness() {
        use crate::tax::types::FilingStatus;
        assert_eq!(
            SKIPPABLE_QUESTIONS.len(),
            16,
            "blind ×2, SALT, DOB ×2, DOD ×2, FBAR, the §G-9 death pair, Schedule C I/J, 8283 5a/5b/5c, \
             8995-A SSTB + patron, §170(f)(8) CWA"
        );
        // SALT is live iff a schedule_a exists; spouse-blind iff a spouse Person exists.
        let salt = SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::SalesTaxElection)
            .unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        assert!(!(salt.live)(&ri));
        ri.schedule_a = Some(Default::default());
        assert!((salt.live)(&ri));
        // The skippables are NOT in FORM_QUESTIONS (merging would brick screen_inputs on a None-legal skippable).
        for s in SKIPPABLE_QUESTIONS {
            assert!(
                !FORM_QUESTIONS
                    .iter()
                    .any(|q| format!("{:?}", q.id) == format!("{:?}", s.id)),
                "a skippable must not also be a mandatory FORM_QUESTIONS declaration"
            );
        }
    }

    /// ★★★ **§G-15 — the year gate, and the reason the always-live workaround had to go.**
    ///
    /// `HasIncomeExclusion` computes modified AGI, which only TY2025+ reads (TY2024's SALT cap is a
    /// `FlatCap` that ignores `magi` entirely). It shipped ALWAYS LIVE because `live` had no year to
    /// consult, so TY2024 filers were asked a TY2025 question — defensible ONLY because a bespoke
    /// neutrality proof existed for that one question.
    ///
    /// ★★ The proof does not generalise, and this test exists so the next year-scoped question is
    /// written as a gate rather than as another workaround: Schedule 1-A Part IV asks about a
    /// deduction that **did not exist in TY2024**, so a "no" there is testimony about nothing.
    #[test]
    fn the_income_exclusion_question_is_live_only_from_ty2025() {
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::HasIncomeExclusion)
            .expect("the question is in the registry");

        let at = |y: i32| ReturnInputs {
            tax_year: y,
            ..Default::default()
        };

        assert!(
            !(q.live)(&at(2024)),
            "TY2024 must NOT be asked a TY2025 MAGI question — that was the §G-15 defect"
        );
        assert!(
            (q.live)(&at(2025)),
            "TY2025 computes modified AGI, so it must be asked"
        );
        assert!((q.live)(&at(2026)), "and every later year");

        // ★ Fail-closed on an unstated year: a fixture that never went through storage has
        // `tax_year: 0`, and 0 must not conjure a TY2025 question out of nothing.
        assert!(
            !(q.live)(&at(0)),
            "an UNSTATED year must not be treated as 2025 — the gate fails closed"
        );
    }

    /// ★★★ **§G-15 — durability, and the rule that decides it.**
    ///
    /// **A question is `Durable` only if its subject cannot change once known.** Two consequences,
    /// and the first is structural rather than a hand-list:
    ///
    /// 1. **No class-(A) DECLARATION may ever be `Durable`.** Every one asserts about a *tax year*
    ///    ("in this tax year, did…"), so last year's answer is not testimony for this one. Marking
    ///    one durable would let a prior year's answer satisfy this year's provenance — the
    ///    answered-ness invariant breached across a year boundary, which is software answering for
    ///    the filer, one year removed.
    /// 2. Among the class-(B) skippables, exactly the **dates of birth** qualify. Blindness can
    ///    change; a §164(b)(5) election is made per year; and a date of DEATH is gated on
    ///    `…DiedDuringYear`, itself a per-year declaration.
    #[test]
    fn only_facts_that_cannot_change_are_durable() {
        let durable_decls: Vec<_> = FORM_QUESTIONS
            .iter()
            .filter(|q| q.durability == Durability::Durable)
            .map(|q| q.id)
            .collect();
        assert!(
            durable_decls.is_empty(),
            "no DECLARATION may be Durable — each asserts about a tax year, so a prior year's answer \
             is not testimony for this one. Offending: {durable_decls:?}"
        );

        let durable_skips: Vec<_> = SKIPPABLE_QUESTIONS
            .iter()
            .filter(|s| s.durability == Durability::Durable)
            .map(|s| s.id)
            .collect();
        assert_eq!(
            durable_skips,
            vec![SkippableId::DobTaxpayer, SkippableId::DobSpouse],
            "exactly the dates of BIRTH are durable. Blindness changes, the sales-tax election is \
             per-year, and a date of DEATH is gated on a per-year declaration — so adding anything \
             here needs an argument that its subject genuinely cannot change"
        );
    }

    /// ★ **The default direction is to RE-ASK.** Anything not deliberately marked durable must be
    /// `PerYear`, so a new registry entry fails toward asking the filer rather than toward reusing
    /// an answer they did not give this year.
    #[test]
    fn everything_not_explicitly_durable_is_per_year() {
        let total = FORM_QUESTIONS.len() + SKIPPABLE_QUESTIONS.len();
        let accounted = FORM_QUESTIONS
            .iter()
            .filter(|q| matches!(q.durability, Durability::PerYear | Durability::Durable))
            .count()
            + SKIPPABLE_QUESTIONS
                .iter()
                .filter(|s| matches!(s.durability, Durability::PerYear | Durability::Durable))
                .count();
        assert_eq!(
            accounted, total,
            "every registry entry must state a durability — the field is not optional, so this can \
             only fail if a variant is added without deciding what it means for carry-forward"
        );
    }
}
