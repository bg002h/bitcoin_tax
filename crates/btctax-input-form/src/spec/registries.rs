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
use btctax_core::tax::questions::{QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS};

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
    }
}
