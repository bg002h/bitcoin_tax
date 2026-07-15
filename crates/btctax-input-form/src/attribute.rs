//! ★ Tier-3 validation (spec §7): the EXHAUSTIVE `attribute(&RefuseReason) -> Vec<Anchor>` map. Every one of
//! the ~37 `screen_inputs` refusals is placed at where in the form it points — a [`Field`], a [`Section`], or
//! [`NotInForm`] (a refusal a v1 form cannot surface: a deferred/TOML-import section or a compute/absolute
//! screen). The `match` has **NO `_` wildcard arm**, so a newly-added `RefuseReason` variant is a compile
//! error until someone places it — the drift guard (spec §7).
//!
//! Declaration anchors resolve through Task 4's [`question_to_field`], so the two spec-§5.8 dedups
//! (`MortgageAllUsedToBuyBuildImprove → SaMortgageAllUsed`, the SALT election → `SaSaltUseSalesTax`) stay
//! automatically correct here — we never hard-code a `Decl*` id that a dedup would have redirected.

use crate::seam::{Anchor, FieldId, SectionId};
use crate::spec::question_to_field;
use btctax_core::tax::questions::QuestionId;
use btctax_core::tax::return_refuse::RefuseReason;

/// The Declaration `Field` carrying `q`, via the Task-4 `QuestionId → FieldId` map (so the mortgage/SALT
/// dedups are honored without a hard-coded `Decl*`).
fn decl(q: QuestionId) -> Anchor {
    Anchor::Field(question_to_field(q))
}

/// Where a screen-refusal points in the input form (spec §7). An EXHAUSTIVE `match` — no `_` arm — so a new
/// `RefuseReason` fails to compile until it is placed. Returns the §7 attribution row's anchor list.
pub fn attribute(r: &RefuseReason) -> Vec<Anchor> {
    use RefuseReason as R;
    match r {
        // ── Unanswered declarations → their Declaration field, exact via QuestionId (§7 line 508). The
        //    mortgage one dedups to the Schedule-A leaf `SaMortgageAllUsed` through `question_to_field`. ──
        R::DependentStatusUnanswered => vec![decl(QuestionId::DependentTaxpayer)],
        R::DependentSpouseStatusUnanswered => vec![decl(QuestionId::DependentSpouse)],
        R::MfsSpouseItemizeUnknown => vec![decl(QuestionId::MfsSpouseItemizes)],
        R::HsaActivityUnanswered => vec![decl(QuestionId::HsaActivity)],
        R::DualStatusAlienUnanswered => vec![decl(QuestionId::DualStatusAlien)],
        R::MixedUseMortgageUnanswered => vec![decl(QuestionId::MortgageAllUsedToBuyBuildImprove)],

        // ── The `Some(true)` value-refusals → the same Declaration field as their unanswered twin (§7 510). ──
        R::ForeignTrust => vec![decl(QuestionId::ForeignTrust)],
        R::HsaActivityUnsupported => vec![decl(QuestionId::HsaActivity)],
        R::DualStatusAlienUnsupported => vec![decl(QuestionId::DualStatusAlien)],
        R::DependentSpouseUnsupported => vec![decl(QuestionId::DependentSpouse)],

        // ── Schedule B Part III is carried by BOTH foreign declarations (I-5) — anchor both; a renderer
        //    focuses the first live-unanswered one (§7 line 509). ──
        R::ScheduleBPart3Unanswered => vec![
            decl(QuestionId::ForeignAccounts),
            decl(QuestionId::ForeignTrust),
        ],
        // Schedule B line 7b — a plain Text leaf (no registry entry), §7 line 511.
        R::ScheduleBForeignCountryMissing => vec![Anchor::Field(FieldId::ForeignCountryNames)],

        // ── Schedule A SALT (§7 lines 512-513). The election's form identity is `SaSaltUseSalesTax` (the
        //    Task-2 dedup — there is NO `SalesTaxElection` FieldId). ──
        R::SaltSalesTaxWithoutElection => vec![
            Anchor::Field(FieldId::SaSaltSalesTaxAmt),
            Anchor::Field(FieldId::SaSaltUseSalesTax),
        ],
        R::SalesTaxElectionWithoutAmount => vec![
            Anchor::Field(FieldId::SaSaltUseSalesTax),
            Anchor::Field(FieldId::SaSaltSalesTaxAmt),
            Anchor::Field(FieldId::SaSaltStateEst),
            Anchor::Field(FieldId::SaSaltPriorYear),
            Anchor::Section(SectionId::W2s),
        ],

        // ── Schedule A charitable (§7 lines 514, 519). ──
        R::NonPublicCharityContribution => vec![
            Anchor::Section(SectionId::ScheduleACharitable),
            Anchor::NotInForm {
                note: "also fires from a non-50%-org charitable carryover-in (`charitable_carryover_in`), a \
                       deferred (non-v1-form) section entered via TOML import (§7 M-3)",
            },
        ],
        R::NonCryptoNoncashGift => vec![Anchor::Section(SectionId::ScheduleACharitable)],

        // ── W-2 sections (§7 lines 515-517). `SingleEmployerExcessSs` is an in-form field, so W2s (I-4). ──
        R::UnsupportedBox12Code(_) => vec![Anchor::Section(SectionId::W2Box12)],
        R::ExcessElectiveDeferral => vec![Anchor::Section(SectionId::W2s)],
        R::AllocatedTips => vec![Anchor::Section(SectionId::W2s)],
        R::DependentCareBenefit => vec![Anchor::Section(SectionId::W2s)],
        R::SingleEmployerExcessSs => vec![Anchor::Section(SectionId::W2s)],

        // ── Spouse-owner: an in-form W-2 leg + a deferred Schedule-C-owner leg (§7 line 518, M-3). ──
        R::SpouseOwnerWithoutJointReturn => vec![
            Anchor::Section(SectionId::W2s),
            Anchor::NotInForm {
                note: "also fires from a spouse-owned Schedule C (`schedule_c.owner`), a deferred \
                       (non-v1-form) section entered via TOML import (§7 M-3)",
            },
        ],

        // ── Defensive-only (§7 line 520): tier-1 parse (Money ≥ 0, `Ssn::canonical`) rejects these before
        //    they can enter the working copy, and the payload is display prose — NOT a field identity that
        //    may be parsed (§7). So the honest anchor is the `NotInForm` sentinel, not a guessed `Field`. ──
        R::NegativeAmount(_) => vec![Anchor::NotInForm {
            note: "defensive only — a negative amount is unreachable from the form: tier-1 parse rejects it \
                   before it enters the working copy, and its label is display prose, not a field identity (§7)",
        }],
        R::SsnMalformed(_) => vec![Anchor::NotInForm {
            note: "defensive only — a malformed SSN is unreachable from the form: tier-1 parse rejects it \
                   before it enters the working copy, and its label names WHO, not a field identity (§7)",
        }],

        // ── Everything else (§7 line 521): a deferred section (Schedule C, QBI, 1099 boxes, carryforwards)
        //    or a compute/absolute screen — no v1 form field to point at. Entered via TOML import or computed
        //    at `report`/`export`. ──
        R::PrivateActivityBondAmt => vec![Anchor::NotInForm {
            note: "private-activity-bond interest (1099-INT box 9 / 1099-DIV box 13) is not a v1 form field — \
                   entered via TOML import (§7 I-3)",
        }],
        R::UnrecapturedOrSpecialRateGain => vec![Anchor::NotInForm {
            note: "special-rate capital gains (1099-DIV box 2b/2c/2d) are not v1 form fields — entered via \
                   TOML import",
        }],
        R::InconsistentDividendSubset(_) => vec![Anchor::NotInForm {
            note: "the 1099-DIV dividend boxes (1a/1b/5) are not v1 form fields — entered via TOML import",
        }],
        R::ForeignTaxOverCeiling => vec![Anchor::NotInForm {
            note: "foreign tax paid (1099-INT box 6 / 1099-DIV box 7) is not a v1 form field — entered via \
                   TOML import",
        }],
        R::IraDeductionClaimed => vec![Anchor::NotInForm {
            note: "the Schedule 1 IRA deduction is not a v1 form field — entered via TOML import",
        }],
        R::BusinessInterestIncome => vec![Anchor::NotInForm {
            note: "business-flagged crypto interest is computed from the ledger, not a v1 form field",
        }],
        R::BusinessIncomeWithoutScheduleC => vec![Anchor::NotInForm {
            note: "SE-eligible business income is computed from the ledger; add a Schedule C via TOML import \
                   (not a v1 form section)",
        }],
        R::ScheduleCLoss => vec![Anchor::NotInForm {
            note: "Schedule C is not a v1 form section — entered via TOML import; a net loss is screened at \
                   `report`",
        }],
        R::ScheduleCNoBusinessDescription => vec![Anchor::NotInForm {
            note: "Schedule C is not a v1 form section — its business description is entered via TOML import",
        }],
        R::KiddieTax => vec![Anchor::NotInForm {
            note: "the §1(g) kiddie-tax screen is computed at `report`, not a v1 form field",
        }],
        R::QbiAboveThreshold => vec![Anchor::NotInForm {
            note: "the §199A QBI-over-threshold screen is computed at `report`, not a v1 form field",
        }],
        R::AmtScreenTriggered => vec![Anchor::NotInForm {
            note: "the Form 6251 AMT screen is computed at `report`, not a v1 form field",
        }],
        R::TaxableIncomeNonPositiveWithCarryforward => vec![Anchor::NotInForm {
            note: "the §1211/§1212 capital-loss-carryover screen is computed at `report`, not a v1 form field",
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seam::Anchor::{Field, NotInForm, Section};

    #[test]
    fn schedule_b_part3_anchors_both_foreign_decls_in_order() {
        assert_eq!(
            attribute(&RefuseReason::ScheduleBPart3Unanswered),
            vec![
                Field(FieldId::DeclForeignAccounts),
                Field(FieldId::DeclForeignTrust),
            ],
        );
    }

    #[test]
    fn single_employer_excess_ss_anchors_the_w2_section() {
        assert_eq!(
            attribute(&RefuseReason::SingleEmployerExcessSs),
            vec![Section(SectionId::W2s)],
        );
    }

    #[test]
    fn private_activity_bond_is_not_in_form() {
        let anchors = attribute(&RefuseReason::PrivateActivityBondAmt);
        assert_eq!(anchors.len(), 1, "exactly one anchor");
        assert!(matches!(anchors[0], NotInForm { .. }), "a 1099 box is not a v1 form field: {anchors:?}");
    }

    #[test]
    fn non_public_charity_has_the_charitable_section_and_a_not_in_form() {
        let anchors = attribute(&RefuseReason::NonPublicCharityContribution);
        assert!(anchors.contains(&Section(SectionId::ScheduleACharitable)), "{anchors:?}");
        assert!(anchors.iter().any(|a| matches!(a, NotInForm { .. })), "carryover-in leg is deferred: {anchors:?}");
    }

    #[test]
    fn non_crypto_noncash_gift_anchors_the_charitable_section_only() {
        assert_eq!(
            attribute(&RefuseReason::NonCryptoNoncashGift),
            vec![Section(SectionId::ScheduleACharitable)],
        );
    }

    /// ★ The SALT dedup (Task-2): the sales-tax election's form identity is the Schedule-A field
    /// `SaSaltUseSalesTax` — there is NO `SalesTaxElection` FieldId. The collapse-guard refusal anchors the
    /// whole income-tax-SALT set, and `SaSaltUseSalesTax` must appear in it.
    #[test]
    fn sales_tax_election_collapse_anchors_the_salt_set_via_the_sa_field() {
        assert_eq!(
            attribute(&RefuseReason::SalesTaxElectionWithoutAmount),
            vec![
                Field(FieldId::SaSaltUseSalesTax),
                Field(FieldId::SaSaltSalesTaxAmt),
                Field(FieldId::SaSaltStateEst),
                Field(FieldId::SaSaltPriorYear),
                Section(SectionId::W2s),
            ],
        );
        // The Schedule-A leaf is the election's form identity — assert it appears (there is no other id to use).
        assert!(attribute(&RefuseReason::SalesTaxElectionWithoutAmount).contains(&Field(FieldId::SaSaltUseSalesTax)));
    }

    /// The other SALT refusal (amount without the election) → the two Schedule-A fields, via the Sa* ids.
    #[test]
    fn salt_amount_without_election_anchors_the_two_schedule_a_fields() {
        assert_eq!(
            attribute(&RefuseReason::SaltSalesTaxWithoutElection),
            vec![
                Field(FieldId::SaSaltSalesTaxAmt),
                Field(FieldId::SaSaltUseSalesTax),
            ],
        );
    }

    /// ★ The mortgage dedup (Task-2): the mixed-use-mortgage declaration's form identity is the Schedule-A
    /// field `SaMortgageAllUsed` — there is NO `DeclMortgageAllUsed`. Resolved via `question_to_field`.
    #[test]
    fn mixed_use_mortgage_unanswered_anchors_the_schedule_a_mortgage_field() {
        assert_eq!(
            attribute(&RefuseReason::MixedUseMortgageUnanswered),
            vec![Field(FieldId::SaMortgageAllUsed)],
        );
    }

    /// The 6 unanswered-declaration refusals resolve to their Declaration field via `QuestionId`.
    #[test]
    fn unanswered_declarations_anchor_their_declaration_field() {
        assert_eq!(attribute(&RefuseReason::DependentStatusUnanswered), vec![Field(FieldId::DeclDependentTaxpayer)]);
        assert_eq!(attribute(&RefuseReason::DependentSpouseStatusUnanswered), vec![Field(FieldId::DeclDependentSpouse)]);
        assert_eq!(attribute(&RefuseReason::MfsSpouseItemizeUnknown), vec![Field(FieldId::DeclMfsSpouseItemizes)]);
        assert_eq!(attribute(&RefuseReason::HsaActivityUnanswered), vec![Field(FieldId::DeclHsaActivity)]);
        assert_eq!(attribute(&RefuseReason::DualStatusAlienUnanswered), vec![Field(FieldId::DeclDualStatusAlien)]);
    }

    /// The `Some(true)` value-refusals anchor the same Declaration field as their unanswered twin (§7 line 510).
    #[test]
    fn value_refusals_anchor_their_declaration_field() {
        assert_eq!(attribute(&RefuseReason::ForeignTrust), vec![Field(FieldId::DeclForeignTrust)]);
        assert_eq!(attribute(&RefuseReason::HsaActivityUnsupported), vec![Field(FieldId::DeclHsaActivity)]);
        assert_eq!(attribute(&RefuseReason::DualStatusAlienUnsupported), vec![Field(FieldId::DeclDualStatusAlien)]);
        assert_eq!(attribute(&RefuseReason::DependentSpouseUnsupported), vec![Field(FieldId::DeclDependentSpouse)]);
    }

    #[test]
    fn box12_and_w2_set_refusals_anchor_the_right_section() {
        assert_eq!(attribute(&RefuseReason::UnsupportedBox12Code("K".into())), vec![Section(SectionId::W2Box12)]);
        assert_eq!(attribute(&RefuseReason::ExcessElectiveDeferral), vec![Section(SectionId::W2s)]);
        assert_eq!(attribute(&RefuseReason::AllocatedTips), vec![Section(SectionId::W2s)]);
        assert_eq!(attribute(&RefuseReason::DependentCareBenefit), vec![Section(SectionId::W2s)]);
    }

    #[test]
    fn spouse_owner_has_the_w2_section_and_a_deferred_leg() {
        let anchors = attribute(&RefuseReason::SpouseOwnerWithoutJointReturn);
        assert_eq!(anchors[0], Section(SectionId::W2s));
        assert!(anchors.iter().any(|a| matches!(a, NotInForm { .. })), "schedule_c.owner leg is deferred: {anchors:?}");
    }

    #[test]
    fn foreign_country_missing_anchors_the_text_field() {
        assert_eq!(attribute(&RefuseReason::ScheduleBForeignCountryMissing), vec![Field(FieldId::ForeignCountryNames)]);
    }

    /// The compute/absolute/deferred bucket (§7 line 521) is all `NotInForm`, plus the defensive-only pair.
    #[test]
    fn deferred_and_defensive_refusals_are_not_in_form() {
        for r in [
            RefuseReason::UnrecapturedOrSpecialRateGain,
            RefuseReason::InconsistentDividendSubset("box 5 §199A dividends".into()),
            RefuseReason::ForeignTaxOverCeiling,
            RefuseReason::IraDeductionClaimed,
            RefuseReason::BusinessInterestIncome,
            RefuseReason::BusinessIncomeWithoutScheduleC,
            RefuseReason::ScheduleCLoss,
            RefuseReason::ScheduleCNoBusinessDescription,
            RefuseReason::KiddieTax,
            RefuseReason::QbiAboveThreshold,
            RefuseReason::AmtScreenTriggered,
            RefuseReason::TaxableIncomeNonPositiveWithCarryforward,
            RefuseReason::NegativeAmount("W-2 box 1 wages".into()),
            RefuseReason::SsnMalformed("taxpayer".into()),
        ] {
            let anchors = attribute(&r);
            assert_eq!(anchors.len(), 1, "{r:?} → exactly one anchor: {anchors:?}");
            assert!(matches!(anchors[0], NotInForm { .. }), "{r:?} must be NotInForm: {anchors:?}");
        }
    }

    /// ★ The invariant behind the whole map: no refusal attributes to nowhere. Every arm returns a non-empty
    /// `Vec<Anchor>`, so a renderer always has something to focus (a field, a section, or an honest note).
    #[test]
    fn every_representative_refusal_yields_a_non_empty_anchor_list() {
        // One representative per §7 anchor family — the compiler's exhaustiveness guarantees the rest.
        for r in [
            RefuseReason::ScheduleBPart3Unanswered,
            RefuseReason::SingleEmployerExcessSs,
            RefuseReason::PrivateActivityBondAmt,
            RefuseReason::NonPublicCharityContribution,
            RefuseReason::NonCryptoNoncashGift,
            RefuseReason::SalesTaxElectionWithoutAmount,
            RefuseReason::SaltSalesTaxWithoutElection,
            RefuseReason::MixedUseMortgageUnanswered,
            RefuseReason::ForeignTrust,
            RefuseReason::UnsupportedBox12Code("K".into()),
        ] {
            assert!(!attribute(&r).is_empty(), "{r:?} must anchor somewhere");
        }
    }
}
