//! ★ The `FormSpec` — the ordered tree of `Section`s over `ReturnInputs` (spec §5). This module assembles
//! the sections; it grows over the plan's tasks 4–5. Task 4 lands the two synthetic registry-driven sections
//! ([`Declarations`](SectionId::Declarations) + [`Skippables`](SectionId::Skippables)); Task 5 prepends the
//! header/W-2/Schedule-A/... sections and re-orders `form_spec()` into the §5.8 render order.
mod registries;
pub use registries::{field_to_question, field_to_skippable, question_to_field, skippable_to_field};

use crate::seam::Section;

/// The v1 `FormSpec`: the sections a renderer walks, in render order. **Grows over tasks 4–5** — Task 5 must
/// insert the header/W-2/Schedule-A/etc. sections *before* these two so the final order matches spec §9A
/// (`… Payments → Declarations → Skippables`).
pub fn form_spec() -> &'static [Section] {
    const SECTIONS: &[Section] = &[registries::DECLARATIONS, registries::SKIPPABLES];
    SECTIONS
}

/// Test helper (shared with Task 5): the section with this id, panicking if absent.
#[cfg(test)]
pub(crate) fn section(id: crate::seam::SectionId) -> &'static Section {
    form_spec()
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("section {id:?} not in form_spec()"))
}

/// Test helper (shared with Task 5): a freshly materialized Single return (the working copy after the
/// filing-status choice, spec §5.7 NI-2).
#[cfg(test)]
pub(crate) fn fresh_single() -> btctax_core::tax::return_inputs::ReturnInputs {
    btctax_core::tax::return_inputs::ReturnInputs {
        filing_status: btctax_core::tax::types::FilingStatus::Single,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seam::{FieldId, FieldValue, RowAddr, SectionId, SetError};
    use btctax_core::tax::questions::{
        QuestionId, SkippableId, SkippableKind, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
    };
    use btctax_core::tax::return_inputs::{Person, ReturnInputs};
    use btctax_core::tax::types::FilingStatus;
    use time::macros::date;

    /// ★ Step 1 (declarations, adjusted per the two-corrections brief). The `Declarations` section holds the
    /// **7** `Decl*` declarations (the 8th — the mortgage box — is deduped to its Schedule-A leaf) plus the
    /// `foreign_country_names` Text field, each delegating to its `FORM_QUESTIONS` entry; and the
    /// `FieldId ↔ QuestionId` map stays TOTAL over all 8 questions (the mortgage one → `SaMortgageAllUsed`).
    #[test]
    fn declarations_section_delegates_seven_decls_and_the_question_map_is_total() {
        let decls = section(SectionId::Declarations);

        // The mortgage declaration is a Schedule-A Field (Task 5), NOT a Declarations Field.
        assert!(
            !decls.fields.iter().any(|f| f.id == FieldId::SaMortgageAllUsed),
            "the mortgage declaration is Schedule-A-owned, not a Declarations Field"
        );

        // Every FORM_QUESTIONS entry whose FieldId is a Decl* appears as exactly one Field here — that's 7.
        let mut decl_count = 0;
        for q in FORM_QUESTIONS {
            if question_to_field(q.id) == FieldId::SaMortgageAllUsed {
                assert!(
                    !decls.fields.iter().any(|f| field_to_question(f.id) == Some(q.id)),
                    "the deduped mortgage declaration must not appear in this section"
                );
                continue;
            }
            decl_count += 1;
            assert_eq!(
                decls.fields.iter().filter(|f| field_to_question(f.id) == Some(q.id)).count(),
                1,
                "declaration {:?} must map to exactly one Declarations Field",
                q.id
            );
        }
        assert_eq!(decl_count, 7, "7 declarations are Decl* fields (the 8th is the mortgage dedup)");

        // 7 delegating Decl* fields + the foreign_country_names Text field.
        assert_eq!(decls.fields.len(), 8, "7 declarations + foreign_country_names");
        assert!(decls.fields.iter().any(|f| f.id == FieldId::ForeignCountryNames));

        // TOTAL, both directions, over all 8 QuestionIds — the mortgage one resolves to SaMortgageAllUsed.
        for q in QuestionId::ALL {
            assert_eq!(field_to_question(question_to_field(*q)), Some(*q), "round-trip {q:?}");
        }
        assert_eq!(
            question_to_field(QuestionId::MortgageAllUsedToBuyBuildImprove),
            FieldId::SaMortgageAllUsed
        );

        // Brief's positive get-delegation check.
        let mut ri = fresh_single();
        ri.foreign_accounts = Some(true);
        let fa = decls.fields.iter().find(|f| f.id == FieldId::DeclForeignAccounts).unwrap();
        assert_eq!((fa.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));

        // Each Decl* Field's live/get/set actually delegate to its registry entry (pins the registry index).
        for f in decls.fields.iter().filter(|f| field_to_question(f.id).is_some()) {
            let q = field_to_question(f.id).unwrap();
            let entry = FORM_QUESTIONS.iter().find(|e| e.id == q).unwrap();

            // live: compare on a Single and an Mfs return (MfsSpouseItemizes' liveness differs there, so a
            // mis-wired index is caught, not just constant-true entries).
            for fs in [FilingStatus::Single, FilingStatus::Mfs] {
                let ri = ReturnInputs { filing_status: fs, ..Default::default() };
                assert_eq!((f.live)(&ri), (entry.live)(&ri), "live delegation {:?} @ {fs:?}", f.id);
            }
            // get delegates: a value written through the registry setter is read back by the Field getter.
            let mut ri = fresh_single();
            (entry.set)(&mut ri, true);
            assert_eq!((f.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));
            // set delegates: a value written through the Field setter is read back by the registry getter.
            let mut ri2 = fresh_single();
            (f.set)(&mut ri2, &RowAddr::default(), FieldValue::TriState(Some(false))).unwrap();
            assert_eq!((entry.get)(&ri2), Some(false), "set delegation {:?}", f.id);
            // wrong FieldValue kind is rejected.
            assert_eq!(
                (f.set)(&mut ri2, &RowAddr::default(), FieldValue::Text("x".into())),
                Err(SetError::WrongKind),
                "wrong-kind set on {:?}",
                f.id
            );
        }
    }

    /// ★ Step 1 (skippables, the parallel test). The `Skippables` section holds exactly the **4** non-SALT
    /// skippables (SALT is deduped to its Schedule-A leaf), each delegating to its `SKIPPABLE_QUESTIONS`
    /// entry; the `FieldId ↔ SkippableId` map stays TOTAL over all 5 skippables (SALT → `SaSaltUseSalesTax`);
    /// and the spouse-gated liveness edge holds.
    #[test]
    fn skippables_section_delegates_four_skippables_and_the_map_is_total() {
        let skips = section(SectionId::Skippables);

        // SALT election is a Schedule-A Field (Task 5), NOT a Skippables Field.
        assert!(
            !skips.fields.iter().any(|f| f.id == FieldId::SaSaltUseSalesTax),
            "the SALT election is Schedule-A-owned, not a Skippables Field"
        );

        // Exactly the four non-SALT skippables.
        let ids: Vec<FieldId> = skips.fields.iter().map(|f| f.id).collect();
        assert_eq!(ids.len(), 4, "blind ×2 + DOB ×2");
        for expected in
            [FieldId::BlindTaxpayer, FieldId::BlindSpouse, FieldId::DobTaxpayer, FieldId::DobSpouse]
        {
            assert!(ids.contains(&expected), "missing skippable field {expected:?}");
        }

        // TOTAL, both directions, over all 5 SkippableIds — SALT resolves to SaSaltUseSalesTax.
        for s in SKIPPABLE_QUESTIONS {
            assert_eq!(field_to_skippable(skippable_to_field(s.id)), Some(s.id), "round-trip {:?}", s.id);
        }
        assert_eq!(skippable_to_field(SkippableId::SalesTaxElection), FieldId::SaSaltUseSalesTax);

        // Each Field's live/get/set delegate to its SKIPPABLE_QUESTIONS entry, by kind.
        for f in skips.fields.iter() {
            let s = field_to_skippable(f.id).unwrap();
            let entry = SKIPPABLE_QUESTIONS.iter().find(|e| e.id == s).unwrap();
            // A spouse-gated skippable needs a spouse present for its setter to stick.
            let seed = |ri: &mut ReturnInputs| {
                if !(entry.live)(ri) {
                    ri.header.spouse = Some(Person::default());
                }
            };
            match entry.kind {
                SkippableKind::YesNo => {
                    let mut ri = fresh_single();
                    seed(&mut ri);
                    (entry.set_bool)(&mut ri, true);
                    assert_eq!((f.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));
                    let mut ri2 = ri.clone();
                    (f.set)(&mut ri2, &RowAddr::default(), FieldValue::TriState(Some(false))).unwrap();
                    assert_eq!((entry.get_bool)(&ri2), Some(false), "set delegation {:?}", f.id);
                    assert_eq!(
                        (f.set)(&mut ri2, &RowAddr::default(), FieldValue::Date(None)),
                        Err(SetError::WrongKind)
                    );
                }
                SkippableKind::Date => {
                    let mut ri = fresh_single();
                    seed(&mut ri);
                    let d = date!(1990 - 01 - 02);
                    (entry.set_date)(&mut ri, d);
                    assert_eq!((f.get)(&ri, &RowAddr::default()), Some(FieldValue::Date(Some(d))));
                    let mut ri2 = ri.clone();
                    let d2 = date!(1985 - 05 - 05);
                    (f.set)(&mut ri2, &RowAddr::default(), FieldValue::Date(Some(d2))).unwrap();
                    assert_eq!((entry.get_date)(&ri2), Some(d2), "set delegation {:?}", f.id);
                    assert_eq!(
                        (f.set)(&mut ri2, &RowAddr::default(), FieldValue::TriState(None)),
                        Err(SetError::WrongKind)
                    );
                }
            }
        }

        // The spouse-gated liveness edge: BlindSpouse is live only when a spouse Person is present.
        let blind_spouse = skips.fields.iter().find(|f| f.id == FieldId::BlindSpouse).unwrap();
        let mut ri = fresh_single();
        assert!(!(blind_spouse.live)(&ri), "BlindSpouse is not live without a spouse");
        ri.header.spouse = Some(Person::default());
        assert!((blind_spouse.live)(&ri), "BlindSpouse is live with a spouse present");
    }
}
