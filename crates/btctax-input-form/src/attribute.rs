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
        // ★★ R10.4 / T4b — both halves of the carried filing status anchor on the field that carries
        //    the CONFIRMATION, not on the status itself: a filer who confirmed by mistake fixes it
        //    here, and one whose status really changed is told where to change it by the refusal's
        //    own text (the Household section / the TOML). Not `NotInForm` — a field exists.
        R::FilingStatusUnconfirmed | R::FilingStatusChanged => {
            vec![decl(QuestionId::FilingStatusConfirmed)]
        }
        // ★★ THE CAPITAL LOSS CARRYOVER WORKSHEET'S TWO HEADER CONDITIONS — unanswered and adverse,
        //    all four anchored on the field that carries the answer.
        //
        // ★★★ **DELIBERATELY NOT `NotInForm`, including for the two ADVERSE ones**, and the reason is
        //     recorded a few screens down on `QbiAboveThreshold`: an anchor saying a refusal has no
        //     form field is a FALSEHOOD when one exists, and it leaves the filer with nowhere to go.
        //     These are v1 form fields (`DECL_FIELDS` indices 15 and 16). A filer who answered YES by
        //     mistake fixes it exactly here; one who answered YES truthfully is told so by the
        //     refusal's TEXT, which names the hand-work — that is the refusal's job, not the anchor's.
        R::JointReturnCarryoverDeclarationUnanswered
        | R::JointReturnCarryoverAttributionUnknown => {
            vec![decl(QuestionId::CarryoverIncludesSpousesJointLoss)]
        }
        R::ExcludedCanceledDebtDeclarationUnanswered
        | R::ExcludedCanceledDebtAttributeReduction => {
            vec![decl(QuestionId::ExcludedCanceledDebt)]
        }
        R::IncomeExclusionUnanswered => vec![decl(QuestionId::HasIncomeExclusion)],
        // ★ spec 1099-DA T6 — the per-(provider, cohort) answers live in the `BrokerReporting` block,
        //   one row per provider, one field per cohort: every broker refusal points at the slot of
        //   its cohort (the row is the provider, resolved by the TUI via `broker_row_provider`).
        //
        // ★★ r3 M-4 — THESE FOUR ANCHORS ARE FORWARD-LOOKING, and that is recorded here rather than
        //    left to be re-discovered. The TUI's commit gate runs `screen_inputs`, and
        //    `screen_broker_reporting` is reached only from `screen_absolute` — which runs only on a
        //    year that COMPUTES (`full_return_for(year)`), and no such year is live today. So no
        //    `CommitOutcome::Refused` can currently carry a broker `RefuseReason`, and
        //    `edit/tax_inputs.rs::focus_refusal` has never been watched moving on one in production.
        //    `attribute()`'s own tests below exercise the mapping in isolation, which cannot see
        //    that; the TUI-side row resolution has its own test. When a live full-return year lands,
        //    the commit gate is the thing to re-check — not these arms.
        R::BrokerReportingUnanswered { cohort, .. }
        | R::BrokerReportingMixed { cohort, .. }
        | R::BrokerBasisDiffers { cohort, .. }
        | R::BrokerAnswerUnread { cohort, .. } => vec![Anchor::Field(match cohort {
            btctax_core::forms::Cohort::Covered => FieldId::BrokerCovered,
            btctax_core::forms::Cohort::Noncovered => FieldId::BrokerNoncovered,
        })],
        // §G-22/B11 — both legs point at the one declaration that decides them.
        R::OtherIncomeUnanswered | R::OtherIncomeOutOfScope => {
            vec![decl(QuestionId::OtherOutOfScopeIncome)]
        }
        // ★★★ FR-29 — Form 8615's three refusals. SKIPPABLES, on the `CooperativePatronUnanswered`
        //     precedent: offered always, mandatory only where the answer changes the number.
        //
        //     ★ NONE of them is `NotInForm`. An input DOES clear each one, and an anchor saying
        //       otherwise is the `QbiAboveThreshold` falsehood recorded below — it leaves the filer
        //       with nowhere to go, and a green test pinned it last time.
        R::Form8615AgeSupportUnanswered => vec![skip(
            btctax_core::tax::questions::SkippableId::Form8615Condition3AgeSupport,
        )],
        R::Form8615ParentAliveUnanswered => vec![skip(
            btctax_core::tax::questions::SkippableId::Form8615Condition4ParentAlive,
        )],
        R::Form8615ParentUnidentifiable => vec![skip(
            btctax_core::tax::questions::SkippableId::Form8615ParentIdentityUnobtainable,
        )],
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
        // ── ★★★ R3 / §5.1 — THE DOCUMENT CENSUS's four refusals. ────────────────────────────────
        //
        // All four point at the census row's OWN field, resolved through `question_to_field` so the
        // anchor cannot drift from the registry.
        //
        // ★★ NONE is `NotInForm`, including the two that are not "unanswered". An anchor claiming a
        //    refusal has no form field is a FALSEHOOD when one exists (the `QbiAboveThreshold` note
        //    below), and every one of these IS clearable in the form: `Unanswered` by answering,
        //    `Contradicted` by removing the rows or flipping the answer, `NotTranscribed` by
        //    entering the document or answering "no", and `Unsupported` by correcting a mistaken
        //    "yes" — a filer who answered it truthfully is told the exit by the refusal's TEXT,
        //    which is the refusal's job and not the anchor's.
        R::DocumentCensusUnanswered { kind }
        | R::DocumentCensusContradicted { kind }
        | R::DocumentDeclaredNotTranscribed { kind }
        | R::DocumentTypeUnsupported { kind } => vec![decl(kind.question_id())],
        R::AmtCarryoverDeclarationUnanswered => vec![decl(QuestionId::AmtCarryoverSameAsRegular)],
        R::AmtDepreciationDeclarationUnanswered => {
            vec![decl(QuestionId::AmtDepreciationSameAsRegular)]
        }

        // ── The `Some(true)` value-refusals → the same Declaration field as their unanswered twin (§7 510). ──
        R::ForeignTrust => vec![decl(QuestionId::ForeignTrust)],
        // ★★★ T16 — `HsaActivityUnsupported` is GONE: an affirmed §223 trigger opens Form 8889.
        //     What anchors here now is the unanswered case of any of Form 8889's own seven
        //     questions, which carries the `QuestionId` and so maps straight back to its leaf.
        R::Form8889Unanswered { question } => vec![decl(*question)],
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
        // ★ Form 8960 line 9b over its §164(b)(6) bound. The amount itself is correctable, and so are
        //   the two Schedule A facts that can zero the bound — the sales-tax election, and the SALT
        //   amounts that decide whether itemizing wins at all.
        R::Nii9bExceedsDeductedSalt => vec![
            Anchor::Field(FieldId::Nii8960Line9b),
            Anchor::Field(FieldId::SaSaltUseSalesTax),
            Anchor::Field(FieldId::SaSaltStateEst),
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
        // ── ★★★ R4 / T5 — THE FIVE RE-ATTRIBUTED ANCHORS. Each of these said *"not a v1 form
        //    field — entered via TOML import"*, which stopped being true the moment the 1099
        //    sections landed. An anchor claiming a refusal has no form field is a FALSEHOOD when one
        //    exists, and it leaves the filer with nowhere to go — the same reasoning already
        //    recorded on `QbiAboveThreshold` and on the carryover-worksheet pair above. ──
        R::PrivateActivityBondAmt => vec![
            Anchor::Field(FieldId::Int1099Box9PrivateActivity),
            Anchor::Field(FieldId::Div1099Box13PrivateActivity),
        ],
        R::UnrecapturedOrSpecialRateGain => vec![
            Anchor::Field(FieldId::Div1099Box2bUnrecap1250),
            Anchor::Field(FieldId::Div1099Box2cSection1202),
            Anchor::Field(FieldId::Div1099Box2dCollectibles),
        ],
        R::InconsistentDividendSubset(_) => vec![
            Anchor::Field(FieldId::Div1099Box1aOrdinary),
            Anchor::Field(FieldId::Div1099Box1bQualified),
            Anchor::Field(FieldId::Div1099Box5Section199a),
        ],
        R::ForeignTaxOverCeiling => vec![
            Anchor::Field(FieldId::Int1099Box6ForeignTax),
            Anchor::Field(FieldId::Div1099Box7ForeignTax),
        ],
        // ── ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Each unanswered leg anchors on its own
        //    declaration; each ADVERSE leg anchors there too, because that is where a filer who
        //    answered by mistake fixes it, and one who answered truthfully is told the exit by the
        //    refusal's own text. ──
        R::WagesWithoutW2Unanswered | R::WagesWithoutW2 => {
            vec![decl(QuestionId::WagesWithoutW2Question)]
        }
        R::InterestOrDividendsWithout1099Unanswered => {
            vec![decl(QuestionId::InterestOrDividendsWithout1099)]
        }
        // ★ The rows themselves are the remedy, so the SECTION is the anchor — and the declaration
        //   beside it, for the filer whose true answer was "no".
        R::FilerRecordsDeclaredNotTranscribed => vec![
            Anchor::Section(SectionId::ScheduleBFilerRecords),
            decl(QuestionId::InterestOrDividendsWithout1099),
        ],
        // ★ The mirror (I-2). Same two anchors, and both are REACHABLE by construction: the
        //   section is live while it has rows precisely so this refusal has an exit, and the
        //   declaration is the other half of the both-ways message.
        R::FilerRecordsContradicted => vec![
            Anchor::Section(SectionId::ScheduleBFilerRecords),
            decl(QuestionId::InterestOrDividendsWithout1099),
        ],
        R::StateRefundWithout1099gUnanswered => {
            vec![decl(QuestionId::StateRefundWithout1099g)]
        }
        R::ItemizedPriorYearUnanswered | R::StateAndLocalRefundWorksheetNotComputed => {
            vec![decl(QuestionId::ItemizedPriorYear)]
        }
        // ── ★★★ R9 / T6 — the Digital Assets question. BOTH legs anchor on the declaration, and
        //    neither is `NotInForm`: the unanswered one is cleared by answering it, and the
        //    CONTRADICTED one is cleared either by answering it the other way (right here) or by
        //    correcting the ledger — and the refusal's own text names the event and that second
        //    exit. An anchor claiming the form cannot reach this would be false. ──
        R::DigitalAssetActivityUnanswered | R::DigitalAssetAnswerContradictsLedger { .. } => {
            vec![decl(QuestionId::DigitalAssetActivity)]
        }
        // ── ★★★ R4 / T5 — the box decisions that refuse. Each anchors on the BOX. ──
        R::AmortizableBondPremiumNotComputed => vec![
            Anchor::Field(FieldId::Int1099Box11BondPremium),
            Anchor::Field(FieldId::Int1099Box12BondPremiumTreasury),
            Anchor::Field(FieldId::Int1099Box13BondPremiumTaxExempt),
        ],
        R::StatutoryEmployeeW2 => vec![Anchor::Field(FieldId::W2Box13StatutoryEmployee)],
        R::FamilyLeaveBenefits => vec![Anchor::Field(FieldId::G1099Box10FamilyLeave)],
        // ★★★ Seam review M-1 — the three "income box with no reader" reasons. Each carries the box
        //   in its payload for the MESSAGE, and anchors on every box that raises it, the same shape
        //   as `AmortizableBondPremiumNotComputed`'s three: the filer's remedy is the box itself.
        R::OtherIncomeLine8zNotModeled(_) => vec![
            Anchor::Field(FieldId::G1099Box5Rtaa),
            Anchor::Field(FieldId::G1099Box6TaxableGrants),
            Anchor::Field(FieldId::B1099Box13Bartering),
        ],
        R::ScheduleFIncomeNotModeled(_) => vec![
            Anchor::Field(FieldId::G1099Box7Agriculture),
            Anchor::Field(FieldId::G1099Box9MarketGain),
        ],
        R::LiquidationDistributionNotComputed(_) => vec![
            Anchor::Field(FieldId::Div1099Box9CashLiquidation),
            Anchor::Field(FieldId::Div1099Box10NoncashLiquidation),
        ],
        // ── ★★★ T16 — Form 8889's five stops. Each anchors on the LEAF the filer must change to
        //    get past it, which for four of the five is the declaration that raised it. ──
        R::HsaLine3WorksheetRequired => vec![
            Anchor::Field(FieldId::DeclHsaEligibleEveryMonth),
            Anchor::Field(FieldId::DeclHsaMedicareEnrollment),
        ],
        R::HsaSeparateForm8889Required => {
            vec![Anchor::Field(FieldId::DeclHsaBothSpousesHaveHsas)]
        }
        // ★ Three leaves raise it: the line-4 declaration and the two documents' own account-type
        //   checkboxes. The payload names WHICH in the message; the anchors offer every remedy.
        R::ArcherOrMaMsaNeedsForm8853(_) => vec![
            Anchor::Field(FieldId::DeclHsaArcherMsaActivity),
            Anchor::Field(FieldId::Sa1099Box5AccountType),
            Anchor::Field(FieldId::Sa5498Box6AccountType),
        ],
        R::SaAccountTypeNotTranscribed(_) => vec![
            Anchor::Field(FieldId::Sa1099Box5AccountType),
            Anchor::Field(FieldId::Sa5498Box6AccountType),
        ],
        R::HsaTestingPeriodFailureNotComputed => {
            vec![Anchor::Field(FieldId::DeclHsaTestingPeriodFailure)]
        }
        // ★★ The two EXCESS rules anchor on the amounts, not on a declaration: the filer's remedy
        //    is to correct the contribution figure (or to withdraw the excess and re-enter it), and
        //    no yes/no answer can clear either.
        R::HsaExcessContributionsNeedForm5329 => vec![
            Anchor::Field(FieldId::HsaLine2Contributions),
            Anchor::Field(FieldId::DeclHsaFamilyCoverage),
        ],
        R::HsaExcessEmployerContributions => vec![
            Anchor::Field(FieldId::HsaEmployerPriorYear),
            Anchor::Field(FieldId::HsaEmployerNextYear),
            Anchor::Field(FieldId::Box12Amount),
        ],
        // ★ The W-2 says the employer contributed and the declaration says nothing happened. Both
        //   leaves are the remedy: answer the §223 question Yes, or correct the box 12 entry.
        R::HsaEmployerContributionWithoutActivity => vec![
            Anchor::Field(FieldId::DeclHsaActivity),
            Anchor::Field(FieldId::Box12Code),
            Anchor::Field(FieldId::Box12Amount),
        ],
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
        // ★ The fifth: the confirmation that clears this refusal is a field ON THE ROW, so the
        //   anchor is that field.
        R::Form1099BNeedsForm8949 => vec![Anchor::Field(FieldId::B1099BasisReportedNoAdjustments)],
        // ★★ T3a — Schedule 1-A lines 5 and 14b. `NotInForm` for a reason that will EXPIRE: the
        //    Sch 1-A form section is not built yet (coverage EXEMPT_PREFIXES says so and why), and
        //    even once it is, the cure for these two is not a Sch 1-A field at all — it is a
        //    1099-NEC / 1099-MISC / 1099-K input surface btctax does not have. Anchoring them at a
        //    field that cannot fix them would be worse than saying plainly where the gap is.
        R::Schedule1aTipsFromTradeOrBusiness => vec![Anchor::NotInForm {
            note: "Schedule 1-A line 5 needs a 1099-NEC / 1099-MISC / 1099-K input surface, which btctax does not have; remove the Schedule C or do not claim Part II",
        }],
        R::Schedule1aOvertimeFromTradeOrBusiness => vec![Anchor::NotInForm {
            note: "Schedule 1-A line 14b needs a 1099-NEC / 1099-MISC input surface, which btctax does not have; remove the Schedule C or do not claim Part III",
        }],
        // ★ Seam review I-4. `NotInForm` for the SAME expiring reason the two above carry: the
        //   Schedule 1-A section is not built yet (`EXEMPT_PREFIXES` says so, and names the task),
        //   so the three conditions live only on the TOML import surface today. Unlike those two,
        //   this one's cure IS a Schedule 1-A field — it becomes `Anchor::Field` when the section
        //   lands, and the coverage KAT will police it then.
        R::QualifiedTipsCautionNotMet => vec![Anchor::NotInForm {
            note: "the Schedule 1-A Part II conditions (occupation_on_treasury_list, excludes_unlisted_occupation_tips, meets_qualified_tip_criteria) are TOML-only until the Sch 1-A form section lands — set them under `[schedule_1a.tips]`, or remove the claim",
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

    /// spec 1099-DA T6 — the four broker refusals anchor at the block's slot for THEIR cohort,
    /// never `NotInForm` any more.
    #[test]
    fn broker_refusals_anchor_the_block_by_cohort() {
        use btctax_core::forms::Cohort;
        let p = || "coinbase".to_string();
        assert_eq!(
            attribute(&RefuseReason::BrokerReportingUnanswered {
                provider: p(),
                cohort: Cohort::Covered,
                year: 2026
            }),
            vec![Field(FieldId::BrokerCovered)]
        );
        assert_eq!(
            attribute(&RefuseReason::BrokerReportingMixed {
                provider: p(),
                cohort: Cohort::Noncovered
            }),
            vec![Field(FieldId::BrokerNoncovered)]
        );
        assert_eq!(
            attribute(&RefuseReason::BrokerBasisDiffers {
                provider: p(),
                cohort: Cohort::Covered
            }),
            vec![Field(FieldId::BrokerCovered)]
        );
        assert_eq!(
            attribute(&RefuseReason::BrokerAnswerUnread {
                provider: p(),
                cohort: Cohort::Noncovered,
                year: 2026
            }),
            vec![Field(FieldId::BrokerNoncovered)]
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

    /// ★★★ **T5 INVERTED THIS TEST, and the inversion IS the deliverable.** It used to pin
    /// *"a 1099 box is not a v1 form field"* — true while the only way in was a TOML table, and a
    /// falsehood the moment the 1099 sections landed. The refusal fires on TWO boxes on TWO
    /// documents, so it anchors on both.
    #[test]
    fn private_activity_bond_anchors_the_two_boxes_that_raise_it() {
        assert_eq!(
            attribute(&RefuseReason::PrivateActivityBondAmt),
            vec![
                Field(FieldId::Int1099Box9PrivateActivity),
                Field(FieldId::Div1099Box13PrivateActivity)
            ],
            "1099-INT box 9 and 1099-DIV box 13 both raise it, and both are form fields now"
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
        // ★★★ T16 — the HSA entry here USED to be `HsaActivityUnsupported`. It is gone: an
        //     affirmed §223 trigger opens Form 8889 rather than refusing it. Its replacement is the
        //     unanswered case of one of Form 8889's own seven questions, which carries the
        //     `QuestionId` and so anchors on a DIFFERENT leaf per question — pinned here on two, so
        //     a payload the map ignored would red.
        assert_eq!(
            attribute(&RefuseReason::Form8889Unanswered {
                question: QuestionId::HsaFamilyCoverage
            }),
            vec![Field(FieldId::DeclHsaFamilyCoverage)]
        );
        assert_eq!(
            attribute(&RefuseReason::Form8889Unanswered {
                question: QuestionId::HsaTestingPeriodFailure
            }),
            vec![Field(FieldId::DeclHsaTestingPeriodFailure)]
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
    ///
    /// ★★★ **T5 REMOVED FOUR ENTRIES FROM THIS LIST** — `UnrecapturedOrSpecialRateGain`,
    /// `InconsistentDividendSubset`, `ForeignTaxOverCeiling` and `Form1099BNeedsForm8949` — because
    /// the 1099-DIV, 1099-INT and 1099-B sections landed and every one of them now has a field the
    /// filer can reach. `IraDeductionClaimed` deliberately STAYS: HSA and IRA contributions are
    /// refused unchanged (§2.2), so its anchor is honest. Their new anchors are asserted in
    /// [`the_five_reattributed_anchors_point_at_real_form_fields`].
    #[test]
    fn deferred_and_defensive_refusals_are_not_in_form() {
        for r in [
            RefuseReason::IraDeductionClaimed,
            RefuseReason::BusinessInterestIncome,
            RefuseReason::BusinessIncomeWithoutScheduleC,
            RefuseReason::ScheduleCLoss,
            RefuseReason::ScheduleCNoBusinessDescription,
            RefuseReason::KiddieTax,
            RefuseReason::AmtScreenTriggered,
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

    /// ★★★ **T5 / R4 — THE `NotInForm` COUNT FELL BY EXACTLY FIVE, and the five are named.**
    ///
    /// The count is READ OFF THIS FILE'S OWN SOURCE, never off a list typed beside it: an anchor
    /// added or removed moves the number with no edit anywhere else, which is the whole point (the
    /// same technique `return_refuse.rs` uses to census its own screen). `Anchor::NotInForm {` is
    /// spelled with its `Anchor::` prefix ONLY inside [`attribute`]; the tests below match on the
    /// bare `NotInForm { .. }` pattern, so they cannot inflate the count.
    ///
    /// ★ The positive half matters more than the number: each of the five now yields anchors that
    ///   are REAL fields of a real section, so the renderer has something to focus and the filer has
    ///   somewhere to go. A count alone could be satisfied by deleting an arm.
    #[test]
    fn the_five_reattributed_anchors_point_at_real_form_fields_and_the_count_fell_by_five() {
        const BEFORE_T5: usize = 17;
        // ★ Counted inside [`attribute`]'s own body only — from `pub fn attribute` to the first
        //   `#[cfg(test)]` — so this test's own prose and its `Anchor::NotInForm { note }` match arm
        //   cannot inflate the number it is asserting.
        let src = include_str!("attribute.rs");
        let start = src
            .find("pub fn attribute(")
            .expect("the function this file exists for");
        let end = src[start..]
            .find("#[cfg(test)]")
            .expect("the test module follows it")
            + start;
        // ★ The seam review's I-4 fold ADDED one — `QualifiedTipsCautionNotMet`, whose three
        //   conditions are TOML-only until the Schedule 1-A section lands. The pin moves
        //   DELIBERATELY and says by how much, so the source stays the counter: a sixth
        //   re-attribution, or a second new `NotInForm` refusal, still reds this.
        const ADDED_BY_I4: usize = 1;
        let now = src[start..end].matches("Anchor::NotInForm {").count();
        assert_eq!(
            now,
            BEFORE_T5 - 5 + ADDED_BY_I4,
            "T5 re-attributed exactly five anchors (PrivateActivityBondAmt, \
             UnrecapturedOrSpecialRateGain, InconsistentDividendSubset, ForeignTaxOverCeiling, \
             Form1099BNeedsForm8949) and the I-4 fold added one (QualifiedTipsCautionNotMet); the \
             source now has {now} `NotInForm` anchors, not {}",
            BEFORE_T5 - 5 + ADDED_BY_I4
        );

        // The five, and every anchor each yields must be a real Field or Section of `form_spec()`.
        for r in [
            RefuseReason::PrivateActivityBondAmt,
            RefuseReason::UnrecapturedOrSpecialRateGain,
            RefuseReason::InconsistentDividendSubset("box 1b qualified dividends".into()),
            RefuseReason::ForeignTaxOverCeiling,
            RefuseReason::Form1099BNeedsForm8949,
        ] {
            let anchors = attribute(&r);
            assert!(!anchors.is_empty(), "{r:?} must anchor somewhere");
            for a in &anchors {
                match a {
                    Anchor::Field(id) => assert!(
                        crate::spec::form_spec()
                            .iter()
                            .any(|s| s.fields.iter().any(|f| f.id == *id)),
                        "{r:?} anchors on {id:?}, which is not a field of the form spec"
                    ),
                    Anchor::Section(sid) => assert!(
                        crate::spec::form_spec().iter().any(|s| s.id == *sid),
                        "{r:?} anchors on {sid:?}, which is not a section of the form spec"
                    ),
                    Anchor::NotInForm { note } => panic!(
                        "{r:?} was re-attributed by T5 and must no longer say it is not in the \
                         form: {note}"
                    ),
                }
            }
        }

        // ★ `IraDeductionClaimed` is NOT among them and must stay `NotInForm`: HSA and IRA
        //   contributions are refused unchanged (§2.2), so there is no field to point at and
        //   inventing one would be the falsehood in the other direction.
        assert!(
            matches!(
                attribute(&RefuseReason::IraDeductionClaimed).as_slice(),
                [NotInForm { .. }]
            ),
            "the Schedule 1 IRA deduction has no form field and its anchor must say so"
        );
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
