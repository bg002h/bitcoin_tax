//! ★ The Declarations + Skippables `FormSpec` sections (spec §5.3 / §5.8) — thin adapters that turn the two
//! CORE question registries ([`FORM_QUESTIONS`], [`SKIPPABLE_QUESTIONS`]) into `FormSpec` `Field`s. Each
//! delegating `Field`'s `live`/`get`/`set` forwards to the registry entry's fn-pointer accessors, so no
//! liveness predicate or accessor is written twice — the "one registry per concept" rule (spec §13) crossing
//! the crate seam. Nothing here names a `ReturnInputs` declaration/skippable field directly; the sole
//! plain-leaf exception is `foreign_country_names`, which has no registry entry (it is a §5.8 Text leaf).
//!
//! ★ **Dedup (the two-corrections interface, mirroring spec §5.8).** The two registry-driven tri-state leaves
//! are Schedule-A-owned (built in Task 5), NOT members of these synthetic sections:
//! `QuestionId::MortgageAllUsedToBuyBuildImprove ↔ FieldId::SaMortgageAllUsed` and
//! `SkippableId::SalesTaxElection ↔ FieldId::SaSaltUseSalesTax`. So `Declarations` drops the mortgage box
//! and `Skippables` drops the SALT election — but the maps below stay **TOTAL** over every question and
//! every skippable (the two deduped ids resolve to their Schedule-A `FieldId`), so Task 9's attribution
//! resolves every one.

use crate::seam::{
    Field, FieldId, FieldKind, FieldValue, Section, SectionId, SectionKind, SetError,
};
use btctax_core::tax::document_census::DocumentRow;
use btctax_core::tax::provenance::DependentGate;
use btctax_core::tax::questions::{
    QuestionId, SkippableId, FORM_QUESTIONS, HOH_MARITAL_BASIS_CHOICES, PARENT_ALIVE_CHOICES,
    SKIPPABLE_QUESTIONS,
};

// ── The delegating-Field generators ──────────────────────────────────────────────────────────────────────
// Each expands to a `Field` whose accessors are NON-CAPTURING closures (a `const` registry path + a literal
// index), which is exactly why they coerce to the bare `fn` pointers `Field` requires — a captured `q` could
// not. The index is a literal so the reference is compile-time; the registry's `QuestionId::ALL`-ordered
// completeness test pins that ordering, and the delegation tests here pin each index → registry entry.

// ★ Each delegating macro takes a leaf-clear closure `|ri| <expr: Result<(), SetError>>` (review I-1): the
// caller names the underlying `Option` leaf to set to `None` (the registry `set` can only write a definite
// yes/no, so it cannot un-answer). The `|$ri:ident|` capture makes the caller's `ri` the SAME token the
// closure binds — macro hygiene otherwise hides the closure param from the passed-in expression.

/// A class-(A) declaration → a `TriState` `Field` over `FORM_QUESTIONS[$idx]`.
macro_rules! decl_tristate {
    ($idx:literal, $fid:expr, |$ri:ident| $clear:expr) => {
        Field {
            id: $fid,
            label: FORM_QUESTIONS[$idx].prompt,
            help: FORM_QUESTIONS[$idx].unanswered_detail,
            kind: FieldKind::TriState,
            live: FORM_QUESTIONS[$idx].live,
            // ★ I-4: a non-live (absent-parent / inapplicable) question reads as `None`, distinct from a
            // live-but-unanswered `Some(TriState(None))` — absent must be distinguishable from unanswered.
            get: |ri, _| {
                if !(FORM_QUESTIONS[$idx].live)(ri) {
                    return None;
                }
                Some(FieldValue::TriState((FORM_QUESTIONS[$idx].get)(ri)))
            },
            // ★ I-4: refuse (`NoSuchRow`) a set on a non-live question rather than silently dropping the
            // write and lying `Ok`. Un-answering (→ `None`) is a `ClearField`, never a `SetField`, so a
            // `TriState(None)` and every non-`TriState` value stay `WrongKind`.
            set: |ri, _, v| {
                if !(FORM_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::TriState(Some(b)) = v else { return Err(SetError::WrongKind) };
                (FORM_QUESTIONS[$idx].set)(ri, b);
                Ok(())
            },
            // ★ I-1 (spec §5.7 M-6): the un-answer path — write the underlying `Option` leaf to `None`.
            clear: Some(|$ri, _| $clear),
        }
    };
}

/// A class-(B) `YesNo` skippable → a `TriState` `Field` over `SKIPPABLE_QUESTIONS[$idx]`.
macro_rules! skippable_tristate {
    ($idx:literal, $fid:expr, |$ri:ident| $clear:expr) => {
        Field {
            id: $fid,
            label: SKIPPABLE_QUESTIONS[$idx].prompt,
            help: SKIPPABLE_QUESTIONS[$idx].help,
            kind: FieldKind::TriState,
            live: SKIPPABLE_QUESTIONS[$idx].live,
            get: |ri, _| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return None;
                }
                Some(FieldValue::TriState((SKIPPABLE_QUESTIONS[$idx].get_bool)(
                    ri,
                )))
            },
            set: |ri, _, v| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::TriState(Some(b)) = v else {
                    return Err(SetError::WrongKind);
                };
                (SKIPPABLE_QUESTIONS[$idx].set_bool)(ri, b);
                Ok(())
            },
            clear: Some(|$ri, _| $clear),
        }
    };
}

/// A class-(B) `Date` skippable → a `Date` `Field` over `SKIPPABLE_QUESTIONS[$idx]`.
macro_rules! skippable_date {
    ($idx:literal, $fid:expr, |$ri:ident| $clear:expr) => {
        Field {
            id: $fid,
            label: SKIPPABLE_QUESTIONS[$idx].prompt,
            help: SKIPPABLE_QUESTIONS[$idx].help,
            kind: FieldKind::Date,
            live: SKIPPABLE_QUESTIONS[$idx].live,
            get: |ri, _| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return None;
                }
                Some(FieldValue::Date((SKIPPABLE_QUESTIONS[$idx].get_date)(ri)))
            },
            set: |ri, _, v| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::Date(Some(d)) = v else {
                    return Err(SetError::WrongKind);
                };
                (SKIPPABLE_QUESTIONS[$idx].set_date)(ri, d);
                Ok(())
            },
            clear: Some(|$ri, _| $clear),
        }
    };
}

/// A class-(B) `Choice` skippable → an `Enum` `Field` over `SKIPPABLE_QUESTIONS[$idx]`.
///
/// ★★★ FR-29 — the ONE question in this registry whose third answer is not "unanswered". `None` still
/// means UNANSWERED (and still refuses); `Some("CannotKnow")` is an ANSWER that opens the SPEC §6.3
/// dead-end path. The two must never collapse, so this macro carries no default and no fallback
/// variant: `set_choice` is a no-op on a string outside the kind's option list, so an unlisted value
/// cannot become an answer.
macro_rules! skippable_choice {
    ($idx:literal, $fid:expr, $options:expr, |$ri:ident| $clear:expr) => {
        Field {
            id: $fid,
            label: SKIPPABLE_QUESTIONS[$idx].prompt,
            help: SKIPPABLE_QUESTIONS[$idx].help,
            kind: FieldKind::Enum($options),
            live: SKIPPABLE_QUESTIONS[$idx].live,
            // ★ A non-live question reads as `None` — absent, distinct from live-but-unanswered.
            //   A live-but-unanswered `Choice` has no `FieldValue` to show either, because
            //   `FieldValue::Choice` is a `String` with no "unanswered" inhabitant; the renderer
            //   distinguishes the two by asking `live` (the same shape `Money` fields already have).
            get: |ri, _| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return None;
                }
                (SKIPPABLE_QUESTIONS[$idx].get_choice)(ri)
                    .map(|c| FieldValue::Choice(c.to_string()))
            },
            set: |ri, _, v| {
                if !(SKIPPABLE_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::Choice(c) = v else {
                    return Err(SetError::WrongKind);
                };
                // ★ An unlisted string is `WrongKind`, never a silent no-op that reports `Ok` — the
                //   caller must see that nothing was stored.
                let Some(opt) = $options.iter().find(|o| **o == c.as_str()) else {
                    return Err(SetError::WrongKind);
                };
                (SKIPPABLE_QUESTIONS[$idx].set_choice)(ri, opt);
                Ok(())
            },
            clear: Some(|$ri, _| $clear),
        }
    };
}

// ── The Declarations section ──────────────────────────────────────────────────────────────────────────────

/// Schedule B line 7b — the one Declarations leaf with NO registry entry (a plain §5.8 Text field). Live only
/// when line 7a is answered Yes, so a "Yes" 7a is answerable in-form (else commit refuses
/// `ScheduleBForeignCountryMissing` with no in-form remedy — spec §5.8).
const FOREIGN_COUNTRY_NAMES: Field = Field {
    id: FieldId::ForeignCountryNames,
    label: "Schedule B line 7b — foreign country name(s)",
    help: "Schedule B Part III line 7b: name the foreign country/countries. Live (and required) only when \
           line 7a — a foreign financial account — is answered Yes.",
    kind: FieldKind::Text,
    live: |ri| ri.foreign_accounts == Some(true),
    get: |ri, _| Some(FieldValue::Text(ri.foreign_country_names.clone())),
    set: |ri, _, v| {
        let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
        ri.foreign_country_names = s;
        Ok(())
    },
    // A plain Text leaf: `apply` clears it via `set(Text(""))`.
    clear: None,
};

/// The 10 delegating declarations (indices 0–6 and 8–10 of `FORM_QUESTIONS`, in `QuestionId::ALL` order;
/// index 7, the mortgage box, is deduped to `SaMortgageAllUsed`) plus the country Text leaf. Counts are
/// ASSERTED in `spec::tests::declarations_section_delegates_every_decl_and_the_question_map_is_total` —
/// this comment is a reader's aid, not the guard.
const DECL_FIELDS: &[Field] = &[
    decl_tristate!(0, FieldId::DeclDependentTaxpayer, |ri| {
        ri.header.can_be_claimed_as_dependent_taxpayer = None;
        Ok(())
    }),
    decl_tristate!(1, FieldId::DeclDependentSpouse, |ri| {
        ri.header.can_be_claimed_as_dependent_spouse = None;
        Ok(())
    }),
    decl_tristate!(2, FieldId::DeclMfsSpouseItemizes, |ri| {
        ri.mfs_spouse_itemizes = None;
        Ok(())
    }),
    decl_tristate!(3, FieldId::DeclForeignAccounts, |ri| {
        ri.foreign_accounts = None;
        Ok(())
    }),
    decl_tristate!(4, FieldId::DeclForeignTrust, |ri| {
        ri.foreign_trust = None;
        Ok(())
    }),
    decl_tristate!(5, FieldId::DeclHsaActivity, |ri| {
        ri.sch1.hsa_activity = None;
        Ok(())
    }),
    decl_tristate!(6, FieldId::DeclDualStatusAlien, |ri| {
        ri.dual_status_alien = None;
        Ok(())
    }),
    // ★ Registry-driven — DELEGATE to `FORM_QUESTIONS` indices 8, 9 and 10 (Form 6251 lines 3, 2k, 2l).
    decl_tristate!(8, FieldId::DeclAmtQualifiedDwelling, |ri| {
        if let Some(a) = ri.schedule_a.as_mut() {
            a.mortgage_dwelling_is_amt_qualified = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    decl_tristate!(9, FieldId::DeclAmtCarryoverSame, |ri| {
        ri.amt_carryover_same_as_regular = None;
        Ok(())
    }),
    decl_tristate!(10, FieldId::DeclAmtDepreciationSame, |ri| {
        ri.amt_depreciation_same_as_regular = None;
        Ok(())
    }),
    // Index 11 — the §911/931/933 exclusion gate. Appended at the END of `FORM_QUESTIONS` on purpose:
    // `decl_tristate!` couples to the ARRAY INDEX, so inserting a question mid-array silently repoints
    // every later entry. (Placing it before `AmtQualifiedDwelling` did exactly that in draft.)
    decl_tristate!(11, FieldId::DeclHasIncomeExclusion, |ri| {
        ri.has_income_exclusion = None;
        Ok(())
    }),
    // Index 12 — the §G-22/B11 scope attestation. Appended at the END for the reason stated above:
    // `decl_tristate!` couples to the ARRAY INDEX, so inserting mid-array silently repoints every
    // later entry.
    decl_tristate!(12, FieldId::DeclOtherOutOfScopeIncome, |ri| {
        ri.other_out_of_scope_income = None;
        Ok(())
    }),
    // Index 14 — Schedule D line 20 / Schedule A line 9's Form 4952 declaration. Appended at the END
    // for the array-index reason above. (Index 13, the §163(h)(3)(B) debt limit, is Schedule-A-owned.)
    decl_tristate!(14, FieldId::DeclFilingForm4952, |ri| {
        ri.filing_form_4952 = None;
        Ok(())
    }),
    // Indices 15 and 16 — the Capital Loss Carryover Worksheet's two unnumbered header conditions.
    // Appended at the END for the array-index reason above.
    decl_tristate!(15, FieldId::DeclCarryoverIncludesSpousesJointLoss, |ri| {
        ri.carryover_includes_spouses_joint_loss = None;
        Ok(())
    }),
    decl_tristate!(16, FieldId::DeclExcludedCanceledDebt, |ri| {
        ri.excluded_canceled_debt = None;
        Ok(())
    }),
    // ★★★ Index 35 — R10.4 / T4b's filing-status confirmation, live only on an OPENED year.
    //     Appended at the END for the array-index reason above; 17..=34 are the census rows.
    decl_tristate!(35, FieldId::DeclFilingStatusConfirmed, |ri| {
        ri.filing_status_confirmed = None;
        Ok(())
    }),
    // ★★★ Indices 36..=39 — R3 / T5's DOCUMENT-LESS INCOME DOOR. Appended at the END for the
    //     array-index reason above. They live in `Declarations` rather than in `DocumentCensus`
    //     because they are not census rows: `row_of_question` returns `None` for them, and they
    //     have no rows to guard against, so the census section's I-10 `ContradictsTranscribedRows`
    //     guard would be meaningless on them.
    decl_tristate!(36, FieldId::DeclWagesWithoutW2, |ri| {
        ri.w2_wages_without_w2 = None;
        Ok(())
    }),
    decl_tristate!(37, FieldId::DeclInterestOrDividendsWithout1099, |ri| {
        ri.interest_or_dividends_without_1099 = None;
        Ok(())
    }),
    decl_tristate!(38, FieldId::DeclStateRefundWithout1099g, |ri| {
        ri.state_refund_without_1099g = None;
        Ok(())
    }),
    decl_tristate!(39, FieldId::DeclItemizedPriorYear, |ri| {
        ri.itemized_prior_year = None;
        Ok(())
    }),
    // ★★★ Index 40 — R9 / T6's DIGITAL ASSETS question, Form 1040 page 1. Appended at the END for
    //     the array-index reason above.
    decl_tristate!(40, FieldId::DeclDigitalAssetActivity, |ri| {
        ri.digital_asset_activity = None;
        Ok(())
    }),
    // ★★★ Indices 43..=49 — T16's FORM 8889 declarations. Appended at the END for the array-index
    //     reason above; 41 and 42 are the two HSA census rows and live in `DocumentCensus`.
    decl_tristate!(43, FieldId::DeclHsaFamilyCoverage, |ri| {
        ri.hsa.family_coverage = None;
        Ok(())
    }),
    decl_tristate!(44, FieldId::DeclHsaEligibleEveryMonth, |ri| {
        ri.hsa.eligible_every_month_same_coverage = None;
        Ok(())
    }),
    decl_tristate!(45, FieldId::DeclHsaAge55OrOlder, |ri| {
        ri.hsa.age_55_or_older_at_year_end = None;
        Ok(())
    }),
    decl_tristate!(46, FieldId::DeclHsaMedicareEnrollment, |ri| {
        ri.hsa.enrolled_in_medicare_any_month = None;
        Ok(())
    }),
    decl_tristate!(47, FieldId::DeclHsaBothSpousesHaveHsas, |ri| {
        ri.hsa.both_spouses_have_hsas = None;
        Ok(())
    }),
    decl_tristate!(48, FieldId::DeclHsaArcherMsaActivity, |ri| {
        ri.hsa.archer_msa_activity = None;
        Ok(())
    }),
    decl_tristate!(49, FieldId::DeclHsaTestingPeriodFailure, |ri| {
        ri.hsa.testing_period_failure = None;
        Ok(())
    }),
    // ★★★ Indices 50 and 51 — the T16 SEAM REVIEW's two declarations (I-3's spouse plan, M-1's
    //     document-less distribution door). Appended at the END for the array-index reason above.
    decl_tristate!(50, FieldId::DeclHsaSpouseFamilyCoverage, |ri| {
        ri.hsa.spouse_family_coverage = None;
        Ok(())
    }),
    decl_tristate!(51, FieldId::DeclHsaDistributionWithout1099sa, |ri| {
        ri.hsa_distribution_without_1099sa = None;
        Ok(())
    }),
    // ★★★ Index 52 — T7 / R6's Step 5 question 1. Appended at the END for the array-index reason above.
    decl_tristate!(52, FieldId::DeclFilerTinIssuedByDueDate, |ri| {
        ri.header.filer_tin_issued_by_due_date = None;
        Ok(())
    }),
    // ★★★ Indices 53..=60 — R7 / T8's HoH and QSS tests, and FR-67's election gate. Appended at the
    //     END for the array-index reason above.
    decl_tristate!(53, FieldId::DeclHohQualifyingPerson, |ri| {
        ri.header.hoh_qualifying_person = None;
        Ok(())
    }),
    decl_tristate!(54, FieldId::DeclHohPaidOverHalfCostOfKeepingUpHome, |ri| {
        ri.header.hoh_paid_over_half_cost_of_keeping_up_home = None;
        Ok(())
    }),
    decl_tristate!(55, FieldId::DeclNraSpouseResidentElection, |ri| {
        ri.header.nra_spouse_resident_election = None;
        Ok(())
    }),
    decl_tristate!(56, FieldId::DeclQssSpouseDiedInWindow, |ri| {
        ri.header.qss_spouse_died_in_window_and_not_remarried = None;
        Ok(())
    }),
    decl_tristate!(57, FieldId::DeclQssChildYouCanClaim, |ri| {
        ri.header.qss_child_you_can_claim = None;
        Ok(())
    }),
    decl_tristate!(58, FieldId::DeclQssChildLivedAllYear, |ri| {
        ri.header.qss_child_lived_in_your_home_all_year = None;
        Ok(())
    }),
    decl_tristate!(59, FieldId::DeclQssPaidOverHalfCost, |ri| {
        ri.header.qss_paid_over_half_cost_of_keeping_up_home = None;
        Ok(())
    }),
    decl_tristate!(60, FieldId::DeclQssCouldHaveFiledJointly, |ri| {
        ri.header.qss_could_have_filed_jointly_in_year_of_death = None;
        Ok(())
    }),
    // ★★★ Index 61 — R8 / T9's Form 8396 gate. Appended at the END for the array-index reason
    //     above; 62..=65 are the sale-of-a-main-home answers, which live in `SectionId::HomeSale`.
    decl_tristate!(61, FieldId::DeclClaimingMortgageInterestCredit, |ri| {
        ri.claiming_mortgage_interest_credit = None;
        Ok(())
    }),
    FOREIGN_COUNTRY_NAMES,
];

pub(crate) const DECLARATIONS: Section = Section {
    id: SectionId::Declarations,
    title: "Declarations",
    kind: SectionKind::Singleton,
    fields: DECL_FIELDS,
};

pub(crate) const INCOME_EXCLUSIONS: Section = Section {
    id: SectionId::IncomeExclusions,
    title: "Income exclusions (\u{a7}911/931/933)",
    kind: SectionKind::Singleton,
    fields: super::sections::INCOME_EXCLUSION_FIELDS,
};

// ── ★★★ R3 / §5.1 — THE DOCUMENT CENSUS section ──────────────────────────────────────────────────

/// A census row → a `TriState` `Field` over `FORM_QUESTIONS[$idx]`, **plus the I-10 guard**.
///
/// Identical to [`decl_tristate!`] except for one thing, and that thing is the point: a `No` while
/// rows of that document are transcribed is REFUSED
/// ([`SetError::ContradictsTranscribedRows`]) instead of being stored. Storing it would leave the
/// return in `RefuseReason::DocumentCensusContradicted` with no in-form remedy; deleting the rows to
/// make the answer true would destroy transcribed testimony on one keystroke. The renderer's exit is
/// *remove N rows and answer No*, with a payload-confirm — the `DeleteSection(ScheduleA)` precedent.
///
/// ★ The guard is on `No` ONLY. A `Yes` never contradicts anything, and clearing the row back to
/// unanswered is always allowed: un-answering asserts nothing.
macro_rules! census_tristate {
    ($idx:literal, $fid:expr, $row:expr) => {
        Field {
            id: $fid,
            label: FORM_QUESTIONS[$idx].prompt,
            help: FORM_QUESTIONS[$idx].unanswered_detail,
            kind: FieldKind::TriState,
            live: FORM_QUESTIONS[$idx].live,
            get: |ri, _| {
                if !(FORM_QUESTIONS[$idx].live)(ri) {
                    return None;
                }
                Some(FieldValue::TriState((FORM_QUESTIONS[$idx].get)(ri)))
            },
            set: |ri, _, v| {
                if !(FORM_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::TriState(Some(b)) = v else {
                    return Err(SetError::WrongKind);
                };
                if !b {
                    if let Some(rows) =
                        btctax_core::tax::document_census::declared_rows(ri, $row)
                    {
                        if rows > 0 {
                            return Err(SetError::ContradictsTranscribedRows { rows });
                        }
                    }
                }
                (FORM_QUESTIONS[$idx].set)(ri, b);
                Ok(())
            },
            // ★ N1 — the SAME liveness guard `get` and `set` carry. It was the one arm of the three
            //   that wrote through on a non-live row. Harmless today (the two non-live rows are
            //   already `None`), but the moment `row_is_live` gains a real predicate — T9's
            //   `schedule_a.is_some()` for `form_1098` — an unguarded `clear` would be a write to a
            //   row the filer is not being asked, on a section they cannot see.
            clear: Some(|ri, _| {
                if !(FORM_QUESTIONS[$idx].live)(ri) {
                    return Err(SetError::NoSuchRow);
                }
                ri.documents.set($row, None);
                Ok(())
            }),
        }
    };
}

/// The eighteen census rows, `FORM_QUESTIONS` indices 17..=34 in `QuestionId::ALL` order.
///
/// ★ The indices are literals because `Field`'s accessors must be `const`/`&'static`, never built by
/// a runtime loop — the same constraint every other registry adapter here lives under. They are
/// pinned by `census_section_delegates_every_row_in_registry_order`.
const DOC_CENSUS_FIELDS: &[Field] = &[
    census_tristate!(17, FieldId::DocW2, DocumentRow::W2),
    census_tristate!(18, FieldId::DocInt1099, DocumentRow::Int1099),
    census_tristate!(19, FieldId::DocDiv1099, DocumentRow::Div1099),
    census_tristate!(20, FieldId::DocB1099, DocumentRow::B1099),
    census_tristate!(21, FieldId::DocG1099, DocumentRow::G1099),
    census_tristate!(22, FieldId::DocForm1098, DocumentRow::Form1098),
    census_tristate!(23, FieldId::DocForm1098e, DocumentRow::Form1098e),
    census_tristate!(41, FieldId::DocSa1099, DocumentRow::Sa1099),
    census_tristate!(42, FieldId::DocSa5498, DocumentRow::Sa5498),
    census_tristate!(24, FieldId::DocR1099, DocumentRow::R1099),
    census_tristate!(25, FieldId::DocSsa1099, DocumentRow::Ssa1099),
    census_tristate!(26, FieldId::DocNecMiscK1099, DocumentRow::NecMiscK1099),
    census_tristate!(27, FieldId::DocK1, DocumentRow::K1),
    census_tristate!(
        28,
        FieldId::DocScheduleERental,
        DocumentRow::ScheduleERental
    ),
    census_tristate!(29, FieldId::DocS1099, DocumentRow::S1099),
    census_tristate!(30, FieldId::DocOid1099, DocumentRow::Oid1099),
    census_tristate!(31, FieldId::DocW2g, DocumentRow::W2g),
    census_tristate!(32, FieldId::DocC1099, DocumentRow::C1099),
    census_tristate!(33, FieldId::DocA1095, DocumentRow::A1095),
    census_tristate!(34, FieldId::DocT1098, DocumentRow::T1098),
];

pub(crate) const DOCUMENT_CENSUS: Section = Section {
    id: SectionId::DocumentCensus,
    title: "Documents received",
    kind: SectionKind::Singleton,
    fields: DOC_CENSUS_FIELDS,
};

// ── The Skippables section ────────────────────────────────────────────────────────────────────────────────

/// The delegating skippables — indices 0, 1, 3..11 of `SKIPPABLE_QUESTIONS` (index 2, the SALT
/// election, is deduped to `SaSaltUseSalesTax`). Equivalent to
/// `SKIPPABLE_QUESTIONS.filter(|s| s.id != SalesTaxElection)`, enumerated by index because `Field`
/// accessors must be `const`/`&'static`, not built by a runtime loop.
const SKIPPABLE_FIELDS: &[Field] = &[
    skippable_tristate!(0, FieldId::BlindTaxpayer, |ri| {
        ri.header.taxpayer.blind = None;
        Ok(())
    }),
    // ★ Parent-gated (spouse): a clear on an absent spouse is `NoSuchRow`, not a silent Ok (I-1/I-4).
    skippable_tristate!(1, FieldId::BlindSpouse, |ri| {
        if let Some(sp) = ri.header.spouse.as_mut() {
            sp.blind = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    skippable_date!(3, FieldId::DobTaxpayer, |ri| {
        ri.header.taxpayer.date_of_birth = None;
        Ok(())
    }),
    skippable_date!(4, FieldId::DobSpouse, |ri| {
        if let Some(sp) = ri.header.spouse.as_mut() {
            sp.date_of_birth = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // §G-9 dates of death — indices 5–6, appended for the same array-index reason as the declarations.
    skippable_date!(5, FieldId::DodTaxpayer, |ri| {
        ri.header.taxpayer.date_of_death = None;
        Ok(())
    }),
    skippable_date!(6, FieldId::DodSpouse, |ri| {
        if let Some(sp) = ri.header.spouse.as_mut() {
            sp.date_of_death = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // ★ Index 7 — Schedule B 7a's FBAR sub-question. Parent-gated on 7a = Yes (the form's own "If
    // 'Yes,'"), so a set/clear while 7a is not Yes is `NoSuchRow` via the macro's liveness check.
    skippable_tristate!(7, FieldId::FbarFilingRequired, |ri| {
        ri.fbar_filing_required = None;
        Ok(())
    }),
    // ★ Indices 8–9 — the §G-9 death gates, MOVED here from `DECL_FIELDS` when they stopped refusing.
    // The taxpayer one is `live: |_| true`, which as a declaration meant it blocked every return.
    skippable_tristate!(8, FieldId::TaxpayerDiedDuringYear, |ri| {
        ri.header.taxpayer_died_during_year = None;
        Ok(())
    }),
    skippable_tristate!(9, FieldId::SpouseDiedDuringYear, |ri| {
        ri.header.spouse_died_during_year = None;
        Ok(())
    }),
    // ★ Indices 10–11 — Schedule C's Form-1099 pair. Parent-gated on a `schedule_c` (and, for line J,
    // on line I being answered YES), so a clear while the parent is absent is `NoSuchRow`.
    skippable_tristate!(10, FieldId::ScheduleC1099Required, |ri| {
        if let Some(c) = ri.schedule_c.as_mut() {
            c.payments_requiring_1099 = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    skippable_tristate!(11, FieldId::ScheduleC1099Filed, |ri| {
        if let Some(c) = ri.schedule_c.as_mut() {
            c.will_file_required_1099 = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // ★ Index 12 — Form 8283 5a/5b/5c as one return-level universal (§G-21). No parent gate: the
    // donations are in the LEDGER, so it is offered always and made mandatory by `screen_absolute`
    // on a return that actually CLAIMS a noncash §170 deduction (Schedule A line 12 > 0).
    skippable_tristate!(12, FieldId::DonationsHadRestrictions, |ri| {
        ri.donations_had_restrictions = None;
        Ok(())
    }),
    // Index 13 — the §G-28/B1b SSTB checkbox, appended at the END for the reason above.
    skippable_tristate!(13, FieldId::ScheduleCIsSstb, |ri| {
        if let Some(c) = ri.schedule_c.as_mut() {
            c.is_sstb = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // Index 14 — the §G-28/B1b cooperative-patron checkbox, likewise appended.
    skippable_tristate!(14, FieldId::ScheduleCIsCooperativePatron, |ri| {
        if let Some(c) = ri.schedule_c.as_mut() {
            c.is_cooperative_patron = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // ★ Index 15 — §170(f)(8)'s contemporaneous written acknowledgment, as one return-level
    //   universal. Same shape as index 12 and for the same reason: no parent gate, because the
    //   donations are in the LEDGER and the §63(e) itemize election is computed. `screen_absolute`
    //   makes it mandatory where the return actually claims the deduction.
    skippable_tristate!(15, FieldId::CharitableCwaObtained, |ri| {
        ri.charitable_cwa_obtained = None;
        Ok(())
    }),
    // ★★★ FR-29 — Form 8615's trio, indices 16–18, appended at the END for the array-index reason
    //     stated above. `skippable_to_field` is exhaustive over `SkippableId`, so the three VARIANTS
    //     are compile-forced; the three literal indices here are the one piece of this change's
    //     bookkeeping the compiler cannot check, which is why the delegation test reads each `Field`
    //     back through its registry entry.
    skippable_tristate!(16, FieldId::Form8615Condition3AgeSupport, |ri| {
        ri.header.form8615_condition3_age_support = None;
        Ok(())
    }),
    skippable_choice!(
        17,
        FieldId::Form8615Condition4ParentAlive,
        PARENT_ALIVE_CHOICES,
        |ri| {
            ri.header.form8615_condition4_parent_alive = None;
            Ok(())
        }
    ),
    // ★★★ The label asks "can you…?" and the leaf records "…unobtainable", so the registry entry's
    //     accessors INVERT. Nothing here re-inverts them: this `Field` delegates, and a second `!`
    //     anywhere on the path would cancel the first.
    skippable_tristate!(18, FieldId::Form8615ParentIdentityUnobtainable, |ri| {
        ri.header.form8615_parent_identity_unobtainable = None;
        Ok(())
    }),
    // ★★★ R7 / T8 — the HoH marital basis, index 19. The registry's SECOND `Choice` and its FIRST
    //     class-(A) entry: `clear` still un-answers it (a filer may correct a mis-click), and the
    //     SCREEN is what makes the blank refuse — the form does not enforce class here, it renders.
    skippable_choice!(
        19,
        FieldId::HohMaritalBasis,
        HOH_MARITAL_BASIS_CHOICES,
        |ri| {
            ri.header.hoh_marital_basis = None;
            Ok(())
        }
    ),
];

pub(crate) const SKIPPABLES: Section = Section {
    id: SectionId::Skippables,
    title: "Skippables",
    kind: SectionKind::Singleton,
    fields: SKIPPABLE_FIELDS,
};

// ── The FieldId ↔ registry-id maps (the one hand-written match, both directions) ──────────────────────────

/// FieldId → its declaration [`QuestionId`], if it is a declaration leaf (else `None`). Reverse of
/// [`question_to_field`]. Consumed by Task 9's attribution (`RefuseReason → QuestionId → FieldId → Anchor`).
pub fn field_to_question(id: FieldId) -> Option<QuestionId> {
    Some(match id {
        FieldId::DeclDependentTaxpayer => QuestionId::DependentTaxpayer,
        FieldId::DeclDependentSpouse => QuestionId::DependentSpouse,
        FieldId::DeclMfsSpouseItemizes => QuestionId::MfsSpouseItemizes,
        FieldId::DeclForeignAccounts => QuestionId::ForeignAccounts,
        FieldId::DeclForeignTrust => QuestionId::ForeignTrust,
        FieldId::DeclHsaActivity => QuestionId::HsaActivity,
        FieldId::DeclDualStatusAlien => QuestionId::DualStatusAlien,
        FieldId::SaMortgageAllUsed => QuestionId::MortgageAllUsedToBuyBuildImprove,
        FieldId::SaMortgageWithinDebtLimit => QuestionId::MortgageWithinDebtLimit,
        FieldId::DeclFilingForm4952 => QuestionId::FilingForm4952,
        FieldId::DeclAmtQualifiedDwelling => QuestionId::AmtQualifiedDwelling,
        FieldId::DeclAmtCarryoverSame => QuestionId::AmtCarryoverSameAsRegular,
        FieldId::DeclAmtDepreciationSame => QuestionId::AmtDepreciationSameAsRegular,
        FieldId::DeclHasIncomeExclusion => QuestionId::HasIncomeExclusion,
        FieldId::DeclOtherOutOfScopeIncome => QuestionId::OtherOutOfScopeIncome,
        FieldId::DeclCarryoverIncludesSpousesJointLoss => {
            QuestionId::CarryoverIncludesSpousesJointLoss
        }
        FieldId::DeclExcludedCanceledDebt => QuestionId::ExcludedCanceledDebt,
        // ★ R3 — the census rows' reverse direction.
        FieldId::DocW2 => QuestionId::DocW2,
        FieldId::DocInt1099 => QuestionId::DocInt1099,
        FieldId::DocDiv1099 => QuestionId::DocDiv1099,
        FieldId::DocB1099 => QuestionId::DocB1099,
        FieldId::DocG1099 => QuestionId::DocG1099,
        FieldId::DocForm1098 => QuestionId::DocForm1098,
        FieldId::DocForm1098e => QuestionId::DocForm1098e,
        FieldId::DocSa1099 => QuestionId::DocSa1099,
        FieldId::DocSa5498 => QuestionId::DocSa5498,
        FieldId::DocR1099 => QuestionId::DocR1099,
        FieldId::DocSsa1099 => QuestionId::DocSsa1099,
        FieldId::DocNecMiscK1099 => QuestionId::DocNecMiscK1099,
        FieldId::DocK1 => QuestionId::DocK1,
        FieldId::DocScheduleERental => QuestionId::DocScheduleERental,
        FieldId::DocS1099 => QuestionId::DocS1099,
        FieldId::DocOid1099 => QuestionId::DocOid1099,
        FieldId::DocW2g => QuestionId::DocW2g,
        FieldId::DocC1099 => QuestionId::DocC1099,
        FieldId::DocA1095 => QuestionId::DocA1095,
        FieldId::DocT1098 => QuestionId::DocT1098,
        FieldId::DeclFilingStatusConfirmed => QuestionId::FilingStatusConfirmed,
        // ★ R3 / T5 — the document-less income door.
        FieldId::DeclWagesWithoutW2 => QuestionId::WagesWithoutW2Question,
        FieldId::DeclInterestOrDividendsWithout1099 => QuestionId::InterestOrDividendsWithout1099,
        FieldId::DeclStateRefundWithout1099g => QuestionId::StateRefundWithout1099g,
        FieldId::DeclItemizedPriorYear => QuestionId::ItemizedPriorYear,
        FieldId::DeclDigitalAssetActivity => QuestionId::DigitalAssetActivity,
        // ★ T16 — Form 8889's seven declarations.
        FieldId::DeclHsaFamilyCoverage => QuestionId::HsaFamilyCoverage,
        FieldId::DeclHsaEligibleEveryMonth => QuestionId::HsaEligibleEveryMonth,
        FieldId::DeclHsaAge55OrOlder => QuestionId::HsaAge55OrOlder,
        FieldId::DeclHsaMedicareEnrollment => QuestionId::HsaMedicareEnrollment,
        FieldId::DeclHsaBothSpousesHaveHsas => QuestionId::HsaBothSpousesHaveHsas,
        FieldId::DeclHsaArcherMsaActivity => QuestionId::HsaArcherMsaActivity,
        FieldId::DeclHsaTestingPeriodFailure => QuestionId::HsaTestingPeriodFailure,
        FieldId::DeclHsaSpouseFamilyCoverage => QuestionId::HsaSpouseFamilyCoverage,
        FieldId::DeclHsaDistributionWithout1099sa => QuestionId::HsaDistributionWithout1099sa,
        // ★ T7 / R6 — Step 5 question 1.
        FieldId::DeclFilerTinIssuedByDueDate => QuestionId::FilerTinIssuedByDueDate,
        // ★ R7 / T8 — HoH, QSS, and FR-67's election gate.
        FieldId::DeclHohQualifyingPerson => QuestionId::HohQualifyingPerson,
        FieldId::DeclHohPaidOverHalfCostOfKeepingUpHome => {
            QuestionId::HohPaidOverHalfCostOfKeepingUpHome
        }
        FieldId::DeclNraSpouseResidentElection => QuestionId::NraSpouseResidentElection,
        FieldId::DeclQssSpouseDiedInWindow => QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        FieldId::DeclQssChildYouCanClaim => QuestionId::QssChildYouCanClaim,
        FieldId::DeclQssChildLivedAllYear => QuestionId::QssChildLivedInYourHomeAllYear,
        FieldId::DeclQssPaidOverHalfCost => QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
        FieldId::DeclQssCouldHaveFiledJointly => QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
        // ★★★ R8 / T9.
        FieldId::DeclClaimingMortgageInterestCredit => QuestionId::ClaimingMortgageInterestCredit,
        FieldId::HomeSaleSoldMainHome => QuestionId::SoldMainHome,
        FieldId::HomeSaleTest1 => QuestionId::HomeSaleTest1OwnedAndLived,
        FieldId::HomeSaleTest2 => QuestionId::HomeSaleTest2NoRecentExclusion,
        FieldId::HomeSaleCanExcludeAllGain => QuestionId::HomeSaleCanExcludeAllGain,
        _ => return None,
    })
}

/// [`QuestionId`] → the FieldId that carries it. **TOTAL** (exhaustive `match`): a new `QuestionId` is a
/// compile error here until mapped. The mortgage declaration is deduped to its Schedule-A leaf (spec §5.8).
pub fn question_to_field(id: QuestionId) -> FieldId {
    match id {
        QuestionId::DependentTaxpayer => FieldId::DeclDependentTaxpayer,
        QuestionId::DependentSpouse => FieldId::DeclDependentSpouse,
        QuestionId::MfsSpouseItemizes => FieldId::DeclMfsSpouseItemizes,
        QuestionId::ForeignAccounts => FieldId::DeclForeignAccounts,
        QuestionId::ForeignTrust => FieldId::DeclForeignTrust,
        QuestionId::HsaActivity => FieldId::DeclHsaActivity,
        QuestionId::DualStatusAlien => FieldId::DeclDualStatusAlien,
        QuestionId::MortgageAllUsedToBuyBuildImprove => FieldId::SaMortgageAllUsed,
        // ★ §163(h)(3)(B) — deduped to its own Schedule-A leaf, like the mixed-use box above: it is a
        //   Schedule-A-owned answer that decides what line 8a may print.
        QuestionId::MortgageWithinDebtLimit => FieldId::SaMortgageWithinDebtLimit,
        // ★ NOT Schedule-A-deduped: the answer governs Schedule D line 20 as well as Schedule A
        //   line 9, and it is live on returns that carry no Schedule A at all.
        QuestionId::FilingForm4952 => FieldId::DeclFilingForm4952,
        // Pure declarations: they carry Form 6251 lines 3, 2k and 2l and print on no Schedule-A line, so
        // they get their own Decl leaves rather than deduping to a Schedule-A field.
        QuestionId::AmtQualifiedDwelling => FieldId::DeclAmtQualifiedDwelling,
        QuestionId::AmtCarryoverSameAsRegular => FieldId::DeclAmtCarryoverSame,
        QuestionId::AmtDepreciationSameAsRegular => FieldId::DeclAmtDepreciationSame,
        QuestionId::HasIncomeExclusion => FieldId::DeclHasIncomeExclusion,
        QuestionId::OtherOutOfScopeIncome => FieldId::DeclOtherOutOfScopeIncome,
        // ★ The Capital Loss Carryover Worksheet's two header conditions. NOT deduped anywhere: the
        //   worksheet is "Keep for Your Records" and prints on no schedule, so like the Form 6251
        //   declarations they get their own `Decl*` leaves.
        QuestionId::CarryoverIncludesSpousesJointLoss => {
            FieldId::DeclCarryoverIncludesSpousesJointLoss
        }
        QuestionId::ExcludedCanceledDebt => FieldId::DeclExcludedCanceledDebt,
        // ★ R3 / §5.1 — the eighteen census rows, each its own leaf in the `DocumentCensus`
        //   section. Nothing to dedup: no other section carries a document-type answer.
        //   DERIVED, not hand-listed — `DocumentRow::question_id` is the one direction and this is
        //   its inverse, so the two cannot drift.
        QuestionId::DocW2 => FieldId::DocW2,
        QuestionId::DocInt1099 => FieldId::DocInt1099,
        QuestionId::DocDiv1099 => FieldId::DocDiv1099,
        QuestionId::DocB1099 => FieldId::DocB1099,
        QuestionId::DocG1099 => FieldId::DocG1099,
        QuestionId::DocForm1098 => FieldId::DocForm1098,
        QuestionId::DocForm1098e => FieldId::DocForm1098e,
        QuestionId::DocSa1099 => FieldId::DocSa1099,
        QuestionId::DocSa5498 => FieldId::DocSa5498,
        QuestionId::DocR1099 => FieldId::DocR1099,
        QuestionId::DocSsa1099 => FieldId::DocSsa1099,
        QuestionId::DocNecMiscK1099 => FieldId::DocNecMiscK1099,
        QuestionId::DocK1 => FieldId::DocK1,
        QuestionId::DocScheduleERental => FieldId::DocScheduleERental,
        QuestionId::DocS1099 => FieldId::DocS1099,
        QuestionId::DocOid1099 => FieldId::DocOid1099,
        QuestionId::DocW2g => FieldId::DocW2g,
        QuestionId::DocC1099 => FieldId::DocC1099,
        QuestionId::DocA1095 => FieldId::DocA1095,
        QuestionId::DocT1098 => FieldId::DocT1098,
        QuestionId::FilingStatusConfirmed => FieldId::DeclFilingStatusConfirmed,
        // ★ R3 / T5 — the document-less income door. Not deduped anywhere: no document section
        //   carries an answer about income that arrived WITHOUT its document.
        QuestionId::WagesWithoutW2Question => FieldId::DeclWagesWithoutW2,
        QuestionId::InterestOrDividendsWithout1099 => FieldId::DeclInterestOrDividendsWithout1099,
        QuestionId::StateRefundWithout1099g => FieldId::DeclStateRefundWithout1099g,
        QuestionId::ItemizedPriorYear => FieldId::DeclItemizedPriorYear,
        // ★ R9 / T6 — Form 1040 page 1's Digital Assets question. Not deduped anywhere: no other
        //   section carries it, and the crypto/1099-DA block asks about BROKER REPORTING, which is
        //   a different question with a different answer space.
        QuestionId::DigitalAssetActivity => FieldId::DeclDigitalAssetActivity,
        // ★ T16 — Form 8889's seven declarations. Not deduped anywhere: the money leaves they gate
        //   live in the `Form8889` section, and none of them is itself an amount.
        QuestionId::HsaFamilyCoverage => FieldId::DeclHsaFamilyCoverage,
        QuestionId::HsaEligibleEveryMonth => FieldId::DeclHsaEligibleEveryMonth,
        QuestionId::HsaAge55OrOlder => FieldId::DeclHsaAge55OrOlder,
        QuestionId::HsaMedicareEnrollment => FieldId::DeclHsaMedicareEnrollment,
        QuestionId::HsaBothSpousesHaveHsas => FieldId::DeclHsaBothSpousesHaveHsas,
        QuestionId::HsaArcherMsaActivity => FieldId::DeclHsaArcherMsaActivity,
        QuestionId::HsaTestingPeriodFailure => FieldId::DeclHsaTestingPeriodFailure,
        QuestionId::HsaSpouseFamilyCoverage => FieldId::DeclHsaSpouseFamilyCoverage,
        QuestionId::HsaDistributionWithout1099sa => FieldId::DeclHsaDistributionWithout1099sa,
        // ★ T7 / R6 — Step 5 question 1. Not deduped anywhere: the Dependents section carries the
        //   PER-ROW gates, and this one is about the filer, not about a row.
        QuestionId::FilerTinIssuedByDueDate => FieldId::DeclFilerTinIssuedByDueDate,
        // ★ R7 / T8 — HoH, QSS, and FR-67's election gate. Not deduped anywhere: no other section
        //   carries a filing-status test, and none of them is an amount.
        QuestionId::HohQualifyingPerson => FieldId::DeclHohQualifyingPerson,
        QuestionId::HohPaidOverHalfCostOfKeepingUpHome => {
            FieldId::DeclHohPaidOverHalfCostOfKeepingUpHome
        }
        QuestionId::NraSpouseResidentElection => FieldId::DeclNraSpouseResidentElection,
        QuestionId::QssSpouseDiedInWindowAndNotRemarried => FieldId::DeclQssSpouseDiedInWindow,
        QuestionId::QssChildYouCanClaim => FieldId::DeclQssChildYouCanClaim,
        QuestionId::QssChildLivedInYourHomeAllYear => FieldId::DeclQssChildLivedAllYear,
        QuestionId::QssPaidOverHalfCostOfKeepingUpHome => FieldId::DeclQssPaidOverHalfCost,
        QuestionId::QssCouldHaveFiledJointlyInYearOfDeath => FieldId::DeclQssCouldHaveFiledJointly,
        // ★★★ R8 / T9 — the Form 8396 gate lives in `Declarations` (it modifies Schedule A line 8a
        //     but is not a Schedule-A leaf: it is a fact about a certificate the filer holds), and
        //     the four sale-of-a-main-home answers live in their own `HomeSale` section.
        QuestionId::ClaimingMortgageInterestCredit => FieldId::DeclClaimingMortgageInterestCredit,
        QuestionId::SoldMainHome => FieldId::HomeSaleSoldMainHome,
        QuestionId::HomeSaleTest1OwnedAndLived => FieldId::HomeSaleTest1,
        QuestionId::HomeSaleTest2NoRecentExclusion => FieldId::HomeSaleTest2,
        QuestionId::HomeSaleCanExcludeAllGain => FieldId::HomeSaleCanExcludeAllGain,
    }
}

/// FieldId → its [`SkippableId`], if it is a skippable leaf (else `None`). Reverse of [`skippable_to_field`].
pub fn field_to_skippable(id: FieldId) -> Option<SkippableId> {
    Some(match id {
        FieldId::BlindTaxpayer => SkippableId::BlindTaxpayer,
        FieldId::BlindSpouse => SkippableId::BlindSpouse,
        FieldId::DobTaxpayer => SkippableId::DobTaxpayer,
        FieldId::DobSpouse => SkippableId::DobSpouse,
        FieldId::DodTaxpayer => SkippableId::DodTaxpayer,
        FieldId::DodSpouse => SkippableId::DodSpouse,
        FieldId::SaSaltUseSalesTax => SkippableId::SalesTaxElection,
        FieldId::FbarFilingRequired => SkippableId::FbarFilingRequired,
        FieldId::TaxpayerDiedDuringYear => SkippableId::TaxpayerDiedDuringYear,
        FieldId::SpouseDiedDuringYear => SkippableId::SpouseDiedDuringYear,
        FieldId::ScheduleC1099Required => SkippableId::ScheduleC1099Required,
        FieldId::ScheduleC1099Filed => SkippableId::ScheduleC1099Filed,
        FieldId::DonationsHadRestrictions => SkippableId::DonationsHadRestrictions,
        FieldId::CharitableCwaObtained => SkippableId::CharitableCwaObtained,
        FieldId::ScheduleCIsSstb => SkippableId::ScheduleCIsSstb,
        FieldId::ScheduleCIsCooperativePatron => SkippableId::ScheduleCIsCooperativePatron,
        FieldId::Form8615Condition3AgeSupport => SkippableId::Form8615Condition3AgeSupport,
        FieldId::Form8615Condition4ParentAlive => SkippableId::Form8615Condition4ParentAlive,
        FieldId::Form8615ParentIdentityUnobtainable => {
            SkippableId::Form8615ParentIdentityUnobtainable
        }
        FieldId::HohMaritalBasis => SkippableId::HohMaritalBasis,
        _ => return None,
    })
}

/// [`SkippableId`] → the FieldId that carries it. **TOTAL** (exhaustive `match`). The SALT election is deduped
/// to its Schedule-A leaf (spec §5.8).
pub fn skippable_to_field(id: SkippableId) -> FieldId {
    match id {
        SkippableId::BlindTaxpayer => FieldId::BlindTaxpayer,
        SkippableId::BlindSpouse => FieldId::BlindSpouse,
        SkippableId::SalesTaxElection => FieldId::SaSaltUseSalesTax,
        SkippableId::DobTaxpayer => FieldId::DobTaxpayer,
        SkippableId::DobSpouse => FieldId::DobSpouse,
        SkippableId::DodTaxpayer => FieldId::DodTaxpayer,
        SkippableId::DodSpouse => FieldId::DodSpouse,
        SkippableId::FbarFilingRequired => FieldId::FbarFilingRequired,
        SkippableId::TaxpayerDiedDuringYear => FieldId::TaxpayerDiedDuringYear,
        SkippableId::SpouseDiedDuringYear => FieldId::SpouseDiedDuringYear,
        SkippableId::ScheduleC1099Required => FieldId::ScheduleC1099Required,
        SkippableId::ScheduleC1099Filed => FieldId::ScheduleC1099Filed,
        SkippableId::DonationsHadRestrictions => FieldId::DonationsHadRestrictions,
        SkippableId::CharitableCwaObtained => FieldId::CharitableCwaObtained,
        SkippableId::ScheduleCIsSstb => FieldId::ScheduleCIsSstb,
        SkippableId::ScheduleCIsCooperativePatron => FieldId::ScheduleCIsCooperativePatron,
        SkippableId::Form8615Condition3AgeSupport => FieldId::Form8615Condition3AgeSupport,
        SkippableId::Form8615Condition4ParentAlive => FieldId::Form8615Condition4ParentAlive,
        SkippableId::Form8615ParentIdentityUnobtainable => {
            FieldId::Form8615ParentIdentityUnobtainable
        }
        SkippableId::HohMaritalBasis => FieldId::HohMaritalBasis,
    }
}

/// ★★★ **T7 / R6 / FR-97 — [`DependentGate`] → the Dependents-section `Field` that carries it.**
/// **TOTAL** (an exhaustive `match`, no `_` arm), exactly like [`question_to_field`] and
/// [`skippable_to_field`]: a new gate is a compile error here until it is placed.
///
/// `DateOfBirth` resolves to the pre-existing `DepDob` leaf rather than a new one — the row already
/// had a date field, and R6 changed its CLASS (required, blocking) rather than adding a second one.
///
/// ★ It lives HERE rather than in `attribute.rs` (where it was written for T7) because it is the
///   third registry↔form map and it now has a second consumer: `apply`'s answer-log key
///   ([`crate::answer_key_for`]) is derived from it, so the gates record on the editor surface the
///   way the declarations and skippables already do.
pub fn gate_to_field(gate: DependentGate) -> FieldId {
    use DependentGate as G;
    match gate {
        G::DateOfBirth => FieldId::DepDob,
        G::QcRelationship => FieldId::DepGateQcRelationship,
        G::YoungerThanYouOrSpouse => FieldId::DepGateYoungerThanYouOrSpouse,
        G::FullTimeStudent => FieldId::DepGateFullTimeStudent,
        G::PermanentlyAndTotallyDisabled => FieldId::DepGatePermanentlyAndTotallyDisabled,
        G::ProvidedOverHalfOwnSupport => FieldId::DepGateProvidedOverHalfOwnSupport,
        G::FilingJointReturn => FieldId::DepGateFilingJointReturn,
        G::JointReturnOnlyToClaimRefund => FieldId::DepGateJointReturnOnlyToClaimRefund,
        G::LivedWithYouOverHalfYear => FieldId::DepGateLivedWithYouOverHalfYear,
        G::LivedWithYouInUs => FieldId::DepGateLivedWithYouInUs,
        G::QualifyingChildOfAnotherPerson => FieldId::DepGateQualifyingChildOfAnotherPerson,
        G::CitizenNationalResidentOrCanadaMexico => {
            FieldId::DepGateCitizenNationalResidentOrCanadaMexico
        }
        G::Married => FieldId::DepGateMarried,
        G::TinIssuedByDueDate => FieldId::DepGateTinIssuedByDueDate,
        G::CitizenNationalOrResidentAlien => FieldId::DepGateCitizenNationalOrResidentAlien,
        G::SsnsValidForEmploymentIssuedByDueDate => {
            FieldId::DepGateSsnsValidForEmploymentIssuedByDueDate
        }
        G::QrRelationshipOrMemberOfHousehold => FieldId::DepGateQrRelationshipOrMemberOfHousehold,
        G::QualifyingChildOfAnyTaxpayer => FieldId::DepGateQualifyingChildOfAnyTaxpayer,
        G::GrossIncomeUnderLimit => FieldId::DepGateGrossIncomeUnderLimit,
        G::YouProvidedOverHalfSupport => FieldId::DepGateYouProvidedOverHalfSupport,
        G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies => {
            FieldId::DepGateDivorcedSeparatedMultipleSupportOrKidnappedRuleApplies
        }
    }
}

/// FieldId → the [`DependentGate`] it carries, if it carries one (else `None`).
///
/// ★★★ **DERIVED as [`gate_to_field`]'s inverse over [`DependentGate::ALL`] — never a second
///     hand-written match.** `field_to_question` / `field_to_skippable` are each a second list that
///     a delegation test has to hold against the first; this one cannot drift, because there is only
///     one list. A twenty-second gate is a compile error in `gate_to_field` and then appears here for
///     free — which is the whole reason FR-97 was a *seam* change rather than twenty new lines.
#[must_use]
pub fn field_to_dependent_gate(id: FieldId) -> Option<DependentGate> {
    DependentGate::ALL
        .iter()
        .copied()
        .find(|g| gate_to_field(*g) == id)
}
