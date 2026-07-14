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
    ];
}

/// Whether Schedule A carries mortgage interest — the mixed-use question's liveness. Deliberately an
/// INPUT predicate (`schedule_a.is_some() ∧ mortgage_interest_1098 > 0`), NOT "Schedule A files" (which is
/// compute-dependent and would brick the standard-deduction-wins filer — §2.7, r3 I-2).
fn mortgage_question_live(ri: &ReturnInputs) -> bool {
    ri.schedule_a
        .as_ref()
        .is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)
}

/// ★ THE REGISTRY. Eight declarations; the liveness lifted from the shipped refusals EXCEPT the two P9
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
    },
    FormQuestion {
        id: QuestionId::ForeignAccounts,
        prompt: "Schedule B line 7a: did you have a financial interest in, or signature authority over, \
                 a FOREIGN financial account?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 7a (a foreign financial account) must be answered on every return — \
             it is the FBAR/FinCEN disclosure, and its own answer is what decides whether Schedule B files \
             (§2.9) — run `btctax income answer`",
        // ★ = §2.9: live ALWAYS. It CANNOT be scoped by `schedule_b_files`, because that predicate reads
        // this very answer — the circular liveness that silently omitted Schedule B in shipped code.
        live: |_ri| true,
        get: |ri| ri.foreign_accounts,
        set: |ri, v| ri.foreign_accounts = Some(v),
    },
    FormQuestion {
        id: QuestionId::ForeignTrust,
        prompt: "Schedule B line 8: did you receive a distribution from — or were you the grantor of, or \
                 transferor to — a FOREIGN TRUST?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 8 (a foreign trust) must be answered on every return — a foreign \
             trust independently requires Part III, so it cannot be scoped by whether Schedule B otherwise \
             files (§2.9) — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.foreign_trust,
        set: |ri, v| ri.foreign_trust = Some(v),
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
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ THE COMPLETENESS ANCHOR (§3.5). Anchored to the ENUM, not to `FORM_QUESTIONS` — an anti-vacuity
    /// test that ITERATED the list would silently drop its own scenario when an entry was dropped (r1 I-4).
    /// The `match` is exhaustive, so a NEW `QuestionId` variant is a COMPILE ERROR until it is listed here;
    /// the index round-trip (r2 M-3) is what stops "add the match arm, skip the `ALL` element", which would
    /// compile green and never be iterated.
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
            };
            assert_eq!(idx, i, "QuestionId::ALL is out of order / missing {id:?}");
            assert_eq!(
                FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(),
                1,
                "exactly one FORM_QUESTIONS entry for {id:?}"
            );
        }
        assert_eq!(QuestionId::ALL.len(), 8, "there are 8 declarations");
        assert_eq!(FORM_QUESTIONS.len(), 8, "one entry per declaration");
    }
}
