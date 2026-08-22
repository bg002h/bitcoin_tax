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

/// ★ A SKIPPABLE's anchor. Needed because §G-21's restriction question is offered as a skippable (the
/// donations are in the ledger, which liveness cannot see) while its REFUSAL is raised by
/// `screen_absolute` — so a screen refusal points at a skippable field, which no other arm
/// does.
fn skip(s: btctax_core::tax::questions::SkippableId) -> Anchor {
    Anchor::Field(crate::spec::skippable_to_field(s))
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
        R::MortgageDebtLimitUnanswered => vec![decl(QuestionId::MortgageWithinDebtLimit)],
        R::Form4952DeclarationUnanswered => vec![decl(QuestionId::FilingForm4952)],
        R::AmtQualifiedDwellingUnanswered => vec![decl(QuestionId::AmtQualifiedDwelling)],
        R::IncomeExclusionUnanswered => vec![decl(QuestionId::HasIncomeExclusion)],
        // §G-22/B11 — both legs point at the one declaration that decides them.
        R::OtherIncomeUnanswered | R::OtherIncomeOutOfScope => {
            vec![decl(QuestionId::OtherOutOfScopeIncome)]
        }
        // §G-28/B1b — SKIPPABLES, offered always and mandatory only where the answer changes the form.
        R::SstbUnanswered => vec![skip(btctax_core::tax::questions::SkippableId::ScheduleCIsSstb)],
        R::CooperativePatronUnanswered => vec![skip(
            btctax_core::tax::questions::SkippableId::ScheduleCIsCooperativePatron,
        )],
        // ★ These three are NOT unanswered questions — the filer answered, and the answer is one
        //   btctax cannot file (a sub-schedule of Form 8995-A it does not fill). No input fixes them,
        //   so there is nothing to point the filer at.
        R::CooperativePatron
        | R::SstbInPhaseInRange
        | R::QbiCarryforwardNeedsSchedule8995AC => vec![],
        R::AmtCarryoverDeclarationUnanswered => vec![decl(QuestionId::AmtCarryoverSameAsRegular)],
        R::AmtDepreciationDeclarationUnanswered => {
            vec![decl(QuestionId::AmtDepreciationSameAsRegular)]
        }

        // ── The `Some(true)` value-refusals → the same Declaration field as their unanswered twin (§7 510). ──
        R::ForeignTrust => vec![decl(QuestionId::ForeignTrust)],
        R::HsaActivityUnsupported => vec![decl(QuestionId::HsaActivity)],
        R::DualStatusAlienUnsupported => vec![decl(QuestionId::DualStatusAlien)],
        R::DependentSpouseUnsupported => vec![decl(QuestionId::DependentSpouse)],
        // Form 6251's two ADVERSE answers: v1 models neither add-back, so each refuses at the same
        // field its unanswered twin anchors.
        // ★ §163(h)(3)(B) answered ADVERSELY. It anchors at the same leaf as its unanswered twin —
        //   the answer IS the input, and the return becomes fileable only by correcting it (or, once
        //   FOLLOWUPS P9(a)/S2 lands, by entering the Pub. 936 worksheet result).
        R::MortgageOverDebtLimit => vec![decl(QuestionId::MortgageWithinDebtLimit)],
        // ★ §163(d) / Form 4952. Both routes to this refusal are correctable in the form — either the
        //   declaration itself, or the Schedule A line-9 amount that broke i4952's exception.
        R::Form4952Required => vec![
            decl(QuestionId::FilingForm4952),
            Anchor::Field(FieldId::SaInvestmentInterest),
        ],
        R::AmtNonQualifiedDwelling => vec![decl(QuestionId::AmtQualifiedDwelling)],
        R::AmtCarryoverDiverges => vec![decl(QuestionId::AmtCarryoverSameAsRegular)],
        R::AmtDepreciationDiverges => vec![decl(QuestionId::AmtDepreciationSameAsRegular)],

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
        R::DonationRestrictionsUnresolved => {
            vec![skip(btctax_core::tax::questions::SkippableId::DonationsHadRestrictions)]
        }
        // ★ §170(f)(8) — both legs (unanswered and "no, I don't hold one") point at the one
        //   skippable that decides them. Same shape as §G-21 directly above: offered always,
        //   mandatory only where `screen_absolute` can see the deduction is actually claimed.
        R::CharitableCwaUnresolved => {
            vec![skip(btctax_core::tax::questions::SkippableId::CharitableCwaObtained)]
        }
        R::NonCryptoNoncashGift => vec![Anchor::Section(SectionId::ScheduleACharitable)],

        // ── W-2 sections (§7 lines 515-517). `SingleEmployerExcessSs` is an in-form field, so W2s (I-4). ──
        R::UnsupportedBox12Code(_) => vec![Anchor::Section(SectionId::W2Box12)],
        R::ExcessElectiveDeferral => vec![Anchor::Section(SectionId::W2s)],
        R::AllocatedTips => vec![Anchor::Section(SectionId::W2s)],
        R::DependentCareBenefit => vec![Anchor::Section(SectionId::W2s)],
        R::SingleEmployerExcessSs => vec![Anchor::Section(SectionId::W2s)],
        // ★ The fix is a missing EIN on a W-2, so the W-2 section is exactly where to send the filer.
        R::ExcessSsEmployerUnknown => vec![Anchor::Section(SectionId::W2s)],

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
        // §G-28/B4 — 1099-B rows are not a v1 form section either; they arrive by TOML import, and the
        // confirmation that fixes this refusal is a field on the row itself.
        R::Form1099BNeedsForm8949 => vec![Anchor::NotInForm {
            note: "Form 1099-B rows are entered via TOML import — set `basis_reported_and_no_adjustments` on the row",
        }],
        R::KiddieTax => vec![Anchor::NotInForm {
            note: "the §1(g) kiddie-tax screen is computed at `report`, not a v1 form field",
        }],
        // ★★★ §G-28/B1b — NO LONGER `NotInForm`, and the change is the whole point of B1b.
        //
        //     This reason used to mean "the 8995-A phase-in is unmodeled and nothing you can enter
        //     will help". It now means precisely "Form 8995-A lines 4 and 7 are unanswered", and both
        //     are v1 form fields — so an anchor saying the refusal has no form field is a FALSEHOOD
        //     that leaves the filer with nowhere to go. It was one, and a green test pinned it.
        R::QbiAboveThreshold => vec![
            Anchor::Field(FieldId::QbiW2Wages),
            Anchor::Field(FieldId::QbiUbia),
        ],
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
        // The §6413(c) EIN refusal is fixed in the same place — a missing `ein` on a W-2.
        assert_eq!(
            attribute(&RefuseReason::ExcessSsEmployerUnknown),
            vec![Section(SectionId::W2s)],
        );
    }

    #[test]
    fn private_activity_bond_is_not_in_form() {
        let anchors = attribute(&RefuseReason::PrivateActivityBondAmt);
        assert_eq!(anchors.len(), 1, "exactly one anchor");
        assert!(
            matches!(anchors[0], NotInForm { .. }),
            "a 1099 box is not a v1 form field: {anchors:?}"
        );
    }

    #[test]
    fn non_public_charity_has_the_charitable_section_and_a_not_in_form() {
        let anchors = attribute(&RefuseReason::NonPublicCharityContribution);
        assert!(
            anchors.contains(&Section(SectionId::ScheduleACharitable)),
            "{anchors:?}"
        );
        assert!(
            anchors.iter().any(|a| matches!(a, NotInForm { .. })),
            "carryover-in leg is deferred: {anchors:?}"
        );
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
        assert!(attribute(&RefuseReason::SalesTaxElectionWithoutAmount)
            .contains(&Field(FieldId::SaSaltUseSalesTax)));
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
        assert_eq!(
            attribute(&RefuseReason::DependentStatusUnanswered),
            vec![Field(FieldId::DeclDependentTaxpayer)]
        );
        assert_eq!(
            attribute(&RefuseReason::DependentSpouseStatusUnanswered),
            vec![Field(FieldId::DeclDependentSpouse)]
        );
        assert_eq!(
            attribute(&RefuseReason::MfsSpouseItemizeUnknown),
            vec![Field(FieldId::DeclMfsSpouseItemizes)]
        );
        assert_eq!(
            attribute(&RefuseReason::HsaActivityUnanswered),
            vec![Field(FieldId::DeclHsaActivity)]
        );
        assert_eq!(
            attribute(&RefuseReason::DualStatusAlienUnanswered),
            vec![Field(FieldId::DeclDualStatusAlien)]
        );
    }

    /// The `Some(true)` value-refusals anchor the same Declaration field as their unanswered twin (§7 line 510).
    #[test]
    fn value_refusals_anchor_their_declaration_field() {
        assert_eq!(
            attribute(&RefuseReason::ForeignTrust),
            vec![Field(FieldId::DeclForeignTrust)]
        );
        assert_eq!(
            attribute(&RefuseReason::HsaActivityUnsupported),
            vec![Field(FieldId::DeclHsaActivity)]
        );
        assert_eq!(
            attribute(&RefuseReason::DualStatusAlienUnsupported),
            vec![Field(FieldId::DeclDualStatusAlien)]
        );
        assert_eq!(
            attribute(&RefuseReason::DependentSpouseUnsupported),
            vec![Field(FieldId::DeclDependentSpouse)]
        );
    }

    #[test]
    fn box12_and_w2_set_refusals_anchor_the_right_section() {
        assert_eq!(
            attribute(&RefuseReason::UnsupportedBox12Code("K".into())),
            vec![Section(SectionId::W2Box12)]
        );
        assert_eq!(
            attribute(&RefuseReason::ExcessElectiveDeferral),
            vec![Section(SectionId::W2s)]
        );
        assert_eq!(
            attribute(&RefuseReason::AllocatedTips),
            vec![Section(SectionId::W2s)]
        );
        assert_eq!(
            attribute(&RefuseReason::DependentCareBenefit),
            vec![Section(SectionId::W2s)]
        );
    }

    #[test]
    fn spouse_owner_has_the_w2_section_and_a_deferred_leg() {
        let anchors = attribute(&RefuseReason::SpouseOwnerWithoutJointReturn);
        assert_eq!(anchors[0], Section(SectionId::W2s));
        assert!(
            anchors.iter().any(|a| matches!(a, NotInForm { .. })),
            "schedule_c.owner leg is deferred: {anchors:?}"
        );
    }

    #[test]
    fn foreign_country_missing_anchors_the_text_field() {
        assert_eq!(
            attribute(&RefuseReason::ScheduleBForeignCountryMissing),
            vec![Field(FieldId::ForeignCountryNames)]
        );
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
            RefuseReason::Form1099BNeedsForm8949,
            RefuseReason::KiddieTax,
            RefuseReason::AmtScreenTriggered,
            RefuseReason::TaxableIncomeNonPositiveWithCarryforward,
            RefuseReason::NegativeAmount("W-2 box 1 wages".into()),
        ] {
            let anchors = attribute(&r);
            assert_eq!(anchors.len(), 1, "{r:?} → exactly one anchor: {anchors:?}");
            assert!(
                matches!(anchors[0], NotInForm { .. }),
                "{r:?} must be NotInForm: {anchors:?}"
            );
        }
    }

    /// ★★★ §G-28/B1b — `QbiAboveThreshold` ANCHORS ON THE TWO FIELDS THAT RESOLVE IT.
    ///
    /// It used to mean "the 8995-A phase-in is unmodeled and nothing you enter will help", and it was
    /// listed above as `NotInForm`. B1b repurposed it to mean exactly "Form 8995-A lines 4 and 7 are
    /// unanswered" — and both ARE v1 form fields. Leaving it in the `NotInForm` list made a GREEN test
    /// pin a falsehood, and left the TUI with nothing to highlight for the one refusal B1b exists to
    /// make fixable.
    #[test]
    fn the_qbi_wage_and_ubia_refusal_points_at_the_two_fields_that_fix_it() {
        let anchors = attribute(&RefuseReason::QbiAboveThreshold);
        assert_eq!(
            anchors,
            vec![
                Anchor::Field(FieldId::QbiW2Wages),
                Anchor::Field(FieldId::QbiUbia)
            ],
            "the refusal must anchor on Form 8995-A lines 4 and 7"
        );
        // …and both really are fields of the form, in a live section — not dangling ids.
        for a in &anchors {
            let Anchor::Field(id) = a else {
                panic!("expected a Field anchor, got {a:?}")
            };
            assert!(
                crate::spec::form_spec()
                    .iter()
                    .any(|s| s.fields.iter().any(|f| f.id == *id)),
                "{id:?} is not a field in the form spec"
            );
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
