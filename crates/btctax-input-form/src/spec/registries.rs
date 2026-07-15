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
//! `SkippableId::SalesTaxElection ↔ FieldId::SaSaltUseSalesTax`. So `Declarations` has 7 delegating fields
//! (+ the country Text field) and `Skippables` has 4 — but the maps below stay **TOTAL** over all 8
//! questions / all 5 skippables (the two deduped ids resolve to their Schedule-A `FieldId`), so Task 9's
//! attribution resolves every one.

use crate::seam::{Field, FieldId, FieldKind, FieldValue, Section, SectionId, SectionKind, SetError};
use btctax_core::tax::questions::{QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS};

// ── The delegating-Field generators ──────────────────────────────────────────────────────────────────────
// Each expands to a `Field` whose accessors are NON-CAPTURING closures (a `const` registry path + a literal
// index), which is exactly why they coerce to the bare `fn` pointers `Field` requires — a captured `q` could
// not. The index is a literal so the reference is compile-time; the registry's `QuestionId::ALL`-ordered
// completeness test pins that ordering, and the delegation tests here pin each index → registry entry.

/// A class-(A) declaration → a `TriState` `Field` over `FORM_QUESTIONS[$idx]`.
macro_rules! decl_tristate {
    ($idx:literal, $fid:expr) => {
        Field {
            id: $fid,
            label: FORM_QUESTIONS[$idx].prompt,
            help: FORM_QUESTIONS[$idx].unanswered_detail,
            kind: FieldKind::TriState,
            live: FORM_QUESTIONS[$idx].live,
            get: |ri, _| Some(FieldValue::TriState((FORM_QUESTIONS[$idx].get)(ri))),
            // The registry setter writes a definite yes/no only; un-answering (→ `None`) is a `ClearField`
            // (Task 7), never a `SetField`. So `TriState(None)` and every non-`TriState` value are rejected.
            set: |ri, _, v| {
                let FieldValue::TriState(Some(b)) = v else { return Err(SetError::WrongKind) };
                (FORM_QUESTIONS[$idx].set)(ri, b);
                Ok(())
            },
        }
    };
}

/// A class-(B) `YesNo` skippable → a `TriState` `Field` over `SKIPPABLE_QUESTIONS[$idx]`.
macro_rules! skippable_tristate {
    ($idx:literal, $fid:expr) => {
        Field {
            id: $fid,
            label: SKIPPABLE_QUESTIONS[$idx].prompt,
            help: SKIPPABLE_QUESTIONS[$idx].help,
            kind: FieldKind::TriState,
            live: SKIPPABLE_QUESTIONS[$idx].live,
            get: |ri, _| Some(FieldValue::TriState((SKIPPABLE_QUESTIONS[$idx].get_bool)(ri))),
            set: |ri, _, v| {
                let FieldValue::TriState(Some(b)) = v else { return Err(SetError::WrongKind) };
                (SKIPPABLE_QUESTIONS[$idx].set_bool)(ri, b);
                Ok(())
            },
        }
    };
}

/// A class-(B) `Date` skippable → a `Date` `Field` over `SKIPPABLE_QUESTIONS[$idx]`.
macro_rules! skippable_date {
    ($idx:literal, $fid:expr) => {
        Field {
            id: $fid,
            label: SKIPPABLE_QUESTIONS[$idx].prompt,
            help: SKIPPABLE_QUESTIONS[$idx].help,
            kind: FieldKind::Date,
            live: SKIPPABLE_QUESTIONS[$idx].live,
            get: |ri, _| Some(FieldValue::Date((SKIPPABLE_QUESTIONS[$idx].get_date)(ri))),
            set: |ri, _, v| {
                let FieldValue::Date(Some(d)) = v else { return Err(SetError::WrongKind) };
                (SKIPPABLE_QUESTIONS[$idx].set_date)(ri, d);
                Ok(())
            },
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
};

/// The 7 delegating declarations (indices 0–6 of `FORM_QUESTIONS`, in `QuestionId::ALL` order; index 7, the
/// mortgage box, is deduped to `SaMortgageAllUsed`) plus the country Text leaf.
const DECL_FIELDS: &[Field] = &[
    decl_tristate!(0, FieldId::DeclDependentTaxpayer),
    decl_tristate!(1, FieldId::DeclDependentSpouse),
    decl_tristate!(2, FieldId::DeclMfsSpouseItemizes),
    decl_tristate!(3, FieldId::DeclForeignAccounts),
    decl_tristate!(4, FieldId::DeclForeignTrust),
    decl_tristate!(5, FieldId::DeclHsaActivity),
    decl_tristate!(6, FieldId::DeclDualStatusAlien),
    FOREIGN_COUNTRY_NAMES,
];

pub(crate) const DECLARATIONS: Section = Section {
    id: SectionId::Declarations,
    title: "Declarations",
    kind: SectionKind::Singleton,
    fields: DECL_FIELDS,
};

// ── The Skippables section ────────────────────────────────────────────────────────────────────────────────

/// The 4 delegating skippables — indices 0, 1, 3, 4 of `SKIPPABLE_QUESTIONS` (index 2, the SALT election, is
/// deduped to `SaSaltUseSalesTax`). Equivalent to `SKIPPABLE_QUESTIONS.filter(|s| s.id != SalesTaxElection)`,
/// enumerated by index because `Field` accessors must be `const`/`&'static`, not built by a runtime loop.
const SKIPPABLE_FIELDS: &[Field] = &[
    skippable_tristate!(0, FieldId::BlindTaxpayer),
    skippable_tristate!(1, FieldId::BlindSpouse),
    skippable_date!(3, FieldId::DobTaxpayer),
    skippable_date!(4, FieldId::DobSpouse),
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
    }
}

/// FieldId → its [`SkippableId`], if it is a skippable leaf (else `None`). Reverse of [`skippable_to_field`].
pub fn field_to_skippable(id: FieldId) -> Option<SkippableId> {
    Some(match id {
        FieldId::BlindTaxpayer => SkippableId::BlindTaxpayer,
        FieldId::BlindSpouse => SkippableId::BlindSpouse,
        FieldId::DobTaxpayer => SkippableId::DobTaxpayer,
        FieldId::DobSpouse => SkippableId::DobSpouse,
        FieldId::SaSaltUseSalesTax => SkippableId::SalesTaxElection,
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
    }
}
