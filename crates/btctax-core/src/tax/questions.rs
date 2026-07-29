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
    ];
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

/// Whether an AMT capital-loss-carryover twin could exist — Form 6251 line 2k's liveness.
fn amt_carryover_question_live(ri: &ReturnInputs) -> bool {
    let cf = ri.capital_loss_carryforward_in;
    cf.short > Usd::ZERO || cf.long > Usd::ZERO
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
             whether a Form 6251 line 2l adjustment is hiding inside it. Most filers answer yes, but the \
             question is asked so a divergent AMT amount — which is an ADD-BACK — is never guessed \
             away. The prompt lists the conditions that permit a yes; if none clearly applies, answer \
             NO. Guessing yes would understate the tax — run `btctax income answer`",
        live: amt_depreciation_question_live,
        get: |ri| ri.amt_depreciation_same_as_regular,
        set: |ri, v| ri.amt_depreciation_same_as_regular = Some(v),
        neutral: true, // "yes, the same" ⇒ Form 6251 line 2l adds nothing back
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

/// ★ THE SKIPPABLE REGISTRY. Five class-(B) prompts — SEPARATE from [`FORM_QUESTIONS`] (spec §5.3). The
/// liveness gates and prompts are lifted verbatim from the old `answer.rs::Skippable`; the `income answer`
/// flow and the form engine both DERIVE their skippable prompts from this one list.
pub const SKIPPABLE_QUESTIONS: &[SkippableQuestion] = &[
    SkippableQuestion {
        id: SkippableId::BlindTaxpayer,
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
            };
            assert_eq!(idx, i, "QuestionId::ALL is out of order / missing {id:?}");
            assert_eq!(
                FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(),
                1,
                "exactly one FORM_QUESTIONS entry for {id:?}"
            );
        }
        assert_eq!(QuestionId::ALL.len(), 11, "there are 11 declarations");
        assert_eq!(FORM_QUESTIONS.len(), 11, "one entry per declaration");
    }

    #[test]
    fn skippable_registry_is_separate_and_has_five_entries_with_correct_liveness() {
        use crate::tax::types::FilingStatus;
        assert_eq!(SKIPPABLE_QUESTIONS.len(), 5, "blind ×2, SALT, DOB ×2");
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
}
