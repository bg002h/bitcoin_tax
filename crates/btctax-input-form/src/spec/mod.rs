//! ★ The `FormSpec` — the ordered tree of `Section`s over `ReturnInputs` (spec §5). This module assembles
//! the sections; it grows over the plan's tasks 4–5. Task 4 lands the two synthetic registry-driven sections
//! ([`Declarations`](SectionId::Declarations) + [`Skippables`](SectionId::Skippables)); Task 5 prepends the
//! header/W-2/Schedule-A/... sections and re-orders `form_spec()` into the §5.8 render order.
#[macro_use]
mod registries;
#[cfg(test)]
mod coverage;
mod sections; // Task 6 — the drift-proofing coverage KAT (spec §5.6).
pub use registries::{
    field_to_question, field_to_skippable, question_to_field, skippable_to_field,
};

use crate::seam::Section;

/// The v1 `FormSpec`: the sections a renderer walks, in spec §9A render order — the
/// header/W-2/Schedule-A/... sections (the `sections` module), then the synthetic registry-driven
/// sections, so the tail is `… Payments → Declarations → DocumentCensus → Skippables`.
///
/// ★ R3 — `DocumentCensus` sits beside `Declarations` (its rows ARE class-(A) declarations) rather
/// than at the top where the interview asks it, because §9A's render order is the seam's contract
/// and re-ordering it is a renderer change, not a spec change.
pub fn form_spec() -> &'static [Section] {
    const SECTIONS: &[Section] = &[
        sections::RETURN_OPTIONS,
        sections::TAXPAYER,
        sections::SPOUSE,
        sections::ADDRESS,
        sections::DEPENDENTS,
        sections::W2S,
        sections::W2_BOX12,
        // ★★★ R4 / T5 — one repeating section per supported information return, in the order a
        //     filer meets the documents (wages, then interest, then dividends, then broker
        //     proceeds, then government payments, then the student-loan statement), with R5's
        //     filer's-records rows immediately after the two 1099s whose door opens them.
        sections::INT_1099S,
        sections::DIV_1099S,
        sections::SCHEDULE_B_FILER_RECORDS,
        sections::B_1099S,
        sections::G_1099S,
        sections::FORM_1098ES,
        // ★ R4 / T16 — the HSA information returns, then Form 8889's own money leaves.
        sections::SA_1099S,
        sections::SA_5498S,
        sections::FORM_8889,
        sections::SCHEDULE_A,
        sections::SCHEDULE_A_CHARITABLE,
        sections::PAYMENTS,
        sections::CARRYFORWARDS,
        sections::QBI_LIMITATION,
        sections::BROKER_REPORTING,
        registries::DECLARATIONS,
        registries::DOCUMENT_CENSUS,
        registries::INCOME_EXCLUSIONS,
        registries::SKIPPABLES,
    ];
    SECTIONS
}

/// ★ spec 1099-DA T6 — the PROVIDER a `BrokerReporting` row stands for (the map's i-th key). The
/// renderer names rows through this rather than reading the map (§9A/§13: it never names a
/// `ReturnInputs` field); the seam owns the row ↔ key correspondence in one place.
pub fn broker_row_provider(
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
    i: usize,
) -> Option<String> {
    ri.broker_reporting.0.keys().nth(i).cloned()
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
    use btctax_core::tax::return_inputs::{ParentAliveAnswer, Person, ReturnInputs};
    use btctax_core::tax::types::FilingStatus;
    use time::macros::date;

    /// ★ Step 1 (declarations, adjusted per the two-corrections brief). The `Declarations` section holds
    /// **15** `Decl*` declarations (the two mortgage boxes are deduped to their Schedule-A leaves) plus
    /// the `foreign_country_names` Text field, each delegating to its `FORM_QUESTIONS` entry; and the
    /// `FieldId ↔ QuestionId` map stays TOTAL over all 17 questions (the mortgage one →
    /// `SaMortgageAllUsed`). Named without a number so the name cannot go stale again.
    #[test]
    fn declarations_section_delegates_every_decl_and_the_question_map_is_total() {
        let decls = section(SectionId::Declarations);
        let sched_a = section(SectionId::ScheduleA);
        let census = section(SectionId::DocumentCensus);

        // ★ The DEDUP set is DERIVED, never hand-listed. A declaration is deduped exactly when
        //   `question_to_field` does not resolve to a Field of this section — so a third Schedule-A
        //   dedup lands in the branch below automatically instead of silently failing the count. (It
        //   was a hand-check on `SaMortgageAllUsed` alone, which went stale the moment §163(h)(3)(B)
        //   added a second Schedule-A-owned declaration.)
        let mut decl_count = 0;
        let mut deduped: Vec<QuestionId> = Vec::new();
        for q in FORM_QUESTIONS {
            let fid = question_to_field(q.id);
            // ★★ R3 — the eighteen DOCUMENT CENSUS rows are declarations too, but they are owned by
            //    their own `DocumentCensus` section (their `set` carries the I-10 rows guard, which
            //    no other declaration has). Their membership is checked below, on the same shape as
            //    the Schedule-A dedup: the section that owns them must have exactly one Field each,
            //    and `Declarations` must not ALSO carry one, or the filer is asked twice.
            if btctax_core::tax::document_census::row_of_question(q.id).is_some() {
                assert!(
                    census.fields.iter().any(|f| f.id == fid),
                    "census declaration {:?} must be a DocumentCensus Field",
                    q.id
                );
                assert!(
                    !decls
                        .fields
                        .iter()
                        .any(|f| field_to_question(f.id) == Some(q.id)),
                    "the census declaration {:?} must not also appear in Declarations",
                    q.id
                );
                continue;
            }
            if !decls.fields.iter().any(|f| f.id == fid) {
                // A deduped declaration must be OWNED by the section that prints its line…
                assert!(
                    sched_a.fields.iter().any(|f| f.id == fid),
                    "deduped declaration {:?} must be a Schedule-A Field",
                    q.id
                );
                // …and must not ALSO appear here, or the filer would be asked it twice.
                assert!(
                    !decls
                        .fields
                        .iter()
                        .any(|f| field_to_question(f.id) == Some(q.id)),
                    "the deduped declaration {:?} must not appear in this section",
                    q.id
                );
                deduped.push(q.id);
                continue;
            }
            decl_count += 1;
            assert_eq!(
                decls
                    .fields
                    .iter()
                    .filter(|f| field_to_question(f.id) == Some(q.id))
                    .count(),
                1,
                "declaration {:?} must map to exactly one Declarations Field",
                q.id
            );
        }
        assert_eq!(
            decl_count, 28,
            "28 declarations are Decl* fields (the other two dedup to Schedule A). ★ R10.4 / T4b \
             added the sixteenth (the carried filing status's confirmation); ★ R3 / T5 added the \
             four of the DOCUMENT-LESS INCOME DOOR — wages with no W-2, interest or dividends with \
             no 1099, a state refund with no 1099-G, and the §111(a) prior-year-itemized gate; \
             ★ R9 / T6 added the twenty-first, Form 1040 page 1's DIGITAL ASSETS question; \
             ★ T16 / FR-76 added SEVEN — Form 8889's coverage box, its line-3 eligibility \
             condition, the age-55 amount, Medicare enrolment, the both-spouses heading condition, \
             the line-4 Archer MSA gate and Part III's testing period."
        );
        assert_eq!(
            deduped,
            vec![
                QuestionId::MortgageAllUsedToBuyBuildImprove,
                QuestionId::MortgageWithinDebtLimit
            ],
            "exactly the two Schedule-A-owned mortgage declarations dedup"
        );

        // 21 delegating Decl* fields + the foreign_country_names Text field.
        assert_eq!(
            decls.fields.len(),
            29,
            "28 declarations + foreign_country_names"
        );
        assert!(decls
            .fields
            .iter()
            .any(|f| f.id == FieldId::ForeignCountryNames));

        // ★ R3 — every census row is a Field of its own section, and the section holds nothing else.
        assert_eq!(
            census.fields.len(),
            btctax_core::tax::document_census::DocumentRow::ALL.len(),
            "one DocumentCensus Field per §5.1 row, and nothing else in the section"
        );
        for row in btctax_core::tax::document_census::DocumentRow::ALL {
            assert_eq!(
                census
                    .fields
                    .iter()
                    .filter(|f| field_to_question(f.id) == Some(row.question_id()))
                    .count(),
                1,
                "census row {row:?} must map to exactly one DocumentCensus Field"
            );
        }

        // TOTAL, both directions, over every QuestionId — the mortgage one resolves to SaMortgageAllUsed.
        for q in QuestionId::ALL {
            assert_eq!(
                field_to_question(question_to_field(*q)),
                Some(*q),
                "round-trip {q:?}"
            );
        }
        assert_eq!(
            question_to_field(QuestionId::MortgageAllUsedToBuyBuildImprove),
            FieldId::SaMortgageAllUsed
        );

        // Brief's positive get-delegation check.
        let mut ri = fresh_single();
        ri.foreign_accounts = Some(true);
        let fa = decls
            .fields
            .iter()
            .find(|f| f.id == FieldId::DeclForeignAccounts)
            .unwrap();
        assert_eq!(
            (fa.get)(&ri, &RowAddr::default()),
            Some(FieldValue::TriState(Some(true)))
        );

        // Each Decl* Field's live/get/set actually delegate to its registry entry (pins the registry index).
        for f in decls
            .fields
            .iter()
            .filter(|f| field_to_question(f.id).is_some())
        {
            let q = field_to_question(f.id).unwrap();
            let entry = FORM_QUESTIONS.iter().find(|e| e.id == q).unwrap();

            // live: compare on a Single and an Mfs return (MfsSpouseItemizes' liveness differs there, so a
            // mis-wired index is caught, not just constant-true entries).
            for fs in [FilingStatus::Single, FilingStatus::Mfs] {
                let ri = ReturnInputs {
                    filing_status: fs,
                    ..Default::default()
                };
                assert_eq!(
                    (f.live)(&ri),
                    (entry.live)(&ri),
                    "live delegation {:?} @ {fs:?}",
                    f.id
                );
            }
            // ★ I-4: the delegating get/set now gate on `live`, so seed a return on which EVERY Decl* is
            // live (Mfs makes `MfsSpouseItemizes` live; a spouse `Person` makes `DependentSpouse` live).
            let live_ri = || {
                let mut ri = ReturnInputs {
                    // ★★ §G-15 — "every Decl* is live" now includes the YEAR-SCOPED ones, so the
                    // seed must state a year. 2025 is the year every registry question is live in;
                    // `Default`'s `0` would silently drop `DeclHasIncomeExclusion` from this
                    // totality check.
                    tax_year: 2025,
                    filing_status: FilingStatus::Mfs,
                    ..Default::default()
                };
                ri.header.spouse = Some(Person::default());
                // Form 6251's three declarations need their own liveness primers: line 3's needs
                // Schedule A mortgage interest, line 2k's a capital-loss carryforward, line 2l's a
                // Schedule C with a nonzero flat expense total.
                ri.schedule_a = Some(btctax_core::tax::return_inputs::ScheduleAInputs {
                    mortgage_interest_1098: rust_decimal_macros::dec!(1),
                    ..Default::default()
                });
                ri.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
                    short: rust_decimal_macros::dec!(1),
                    long: rust_decimal_macros::dec!(0),
                };
                ri.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
                    expenses: rust_decimal_macros::dec!(1),
                    ..Default::default()
                });
                // ★ R10.4 / T4b — liveness primer for the carried filing status's confirmation: it
                //   is live only on a year the OPENER made.
                ri.opened_from = Some(2024);
                // ★★★ R3 / T5 — the DOCUMENT-LESS INCOME DOOR's four declarations. Each of the
                //     first three is live EXACTLY when its census row says `No`, and the fourth
                //     (§111(a)) once a refund exists — so the primer answers every countable row
                //     `No` and gives the fixture a 1099-G box-2 refund from the DOCUMENT side.
                //     Priming all four at once is safe here because the test drives one field at a
                //     time through the registry, never the census.
                for row in btctax_core::tax::document_census::DocumentRow::ALL {
                    ri.documents.set(*row, Some(false));
                }
                ri.g_1099 = vec![btctax_core::tax::return_inputs::Form1099G {
                    box2_state_refund: rust_decimal_macros::dec!(1),
                    ..Default::default()
                }];
                // ★★★ T16 — the liveness primer for Form 8889's seven declarations, which share
                //     one predicate: the §223 trigger is affirmed. Safe to prime here for the same
                //     reason the census rows are — the test drives one field at a time through the
                //     registry, and `DeclHsaActivity` is itself driven by the loop above.
                ri.sch1.hsa_activity = Some(true);
                ri
            };
            assert!(
                (entry.live)(&live_ri()),
                "test fixture must be live for {:?}",
                f.id
            );
            // get delegates: a value written through the registry setter is read back by the Field getter.
            let mut ri = live_ri();
            (entry.set)(&mut ri, true);
            assert_eq!(
                (f.get)(&ri, &RowAddr::default()),
                Some(FieldValue::TriState(Some(true)))
            );
            // set delegates: a value written through the Field setter is read back by the registry getter.
            let mut ri2 = live_ri();
            (f.set)(
                &mut ri2,
                &RowAddr::default(),
                FieldValue::TriState(Some(false)),
            )
            .unwrap();
            assert_eq!((entry.get)(&ri2), Some(false), "set delegation {:?}", f.id);
            // wrong FieldValue kind is rejected.
            assert_eq!(
                (f.set)(&mut ri2, &RowAddr::default(), FieldValue::Text("x".into())),
                Err(SetError::WrongKind),
                "wrong-kind set on {:?}",
                f.id
            );
            // ★ I-4: a set on a NON-live question refuses (`NoSuchRow`), not a silent Ok.
            if !(entry.live)(&fresh_single()) {
                let mut dead = fresh_single();
                assert_eq!(
                    (f.set)(
                        &mut dead,
                        &RowAddr::default(),
                        FieldValue::TriState(Some(true))
                    ),
                    Err(SetError::NoSuchRow),
                    "set on non-live {:?} must refuse",
                    f.id
                );
                assert_eq!(
                    (f.get)(&dead, &RowAddr::default()),
                    None,
                    "get on non-live {:?} is None",
                    f.id
                );
            }
        }
    }

    /// ★ Step 1 (skippables, the parallel test). The `Skippables` section holds exactly the **4** non-SALT
    /// skippables (SALT is deduped to its Schedule-A leaf), each delegating to its `SKIPPABLE_QUESTIONS`
    /// entry; the `FieldId ↔ SkippableId` map stays TOTAL over all 5 skippables (SALT → `SaSaltUseSalesTax`);
    /// and the spouse-gated liveness edge holds.
    #[test]
    fn skippables_section_delegates_every_non_salt_skippable_and_the_map_is_total() {
        let skips = section(SectionId::Skippables);

        // SALT election is a Schedule-A Field (Task 5), NOT a Skippables Field.
        assert!(
            !skips
                .fields
                .iter()
                .any(|f| f.id == FieldId::SaSaltUseSalesTax),
            "the SALT election is Schedule-A-owned, not a Skippables Field"
        );

        // ★ EXACTLY the non-SALT skippables — DERIVED from the registry, not a hand-count, so a new
        //   `SKIPPABLE_QUESTIONS` entry that never reached this section fails HERE by name rather
        //   than by an off-by-one on a literal somebody has to remember to bump.
        let ids: Vec<FieldId> = skips.fields.iter().map(|f| f.id).collect();
        let expected_here: Vec<FieldId> = SKIPPABLE_QUESTIONS
            .iter()
            .map(|s| skippable_to_field(s.id))
            .filter(|f| *f != FieldId::SaSaltUseSalesTax)
            .collect();
        assert_eq!(
            ids, expected_here,
            "the Skippables section must hold exactly the non-SALT skippables, in registry order"
        );
        for expected in [
            FieldId::BlindTaxpayer,
            FieldId::BlindSpouse,
            FieldId::DobTaxpayer,
            FieldId::DobSpouse,
            FieldId::DodTaxpayer,
            FieldId::DodSpouse,
            FieldId::FbarFilingRequired,
            FieldId::TaxpayerDiedDuringYear,
            FieldId::SpouseDiedDuringYear,
            FieldId::ScheduleC1099Required,
            FieldId::ScheduleC1099Filed,
            FieldId::DonationsHadRestrictions,
            FieldId::ScheduleCIsSstb,
            FieldId::ScheduleCIsCooperativePatron,
            FieldId::CharitableCwaObtained,
            FieldId::Form8615Condition3AgeSupport,
            FieldId::Form8615Condition4ParentAlive,
            FieldId::Form8615ParentIdentityUnobtainable,
        ] {
            assert!(
                ids.contains(&expected),
                "missing skippable field {expected:?}"
            );
        }

        // TOTAL, both directions, over all 7 SkippableIds — SALT resolves to SaSaltUseSalesTax.
        for s in SKIPPABLE_QUESTIONS {
            assert_eq!(
                field_to_skippable(skippable_to_field(s.id)),
                Some(s.id),
                "round-trip {:?}",
                s.id
            );
        }
        assert_eq!(
            skippable_to_field(SkippableId::SalesTaxElection),
            FieldId::SaSaltUseSalesTax
        );

        // Each Field's live/get/set delegate to its SKIPPABLE_QUESTIONS entry, by kind.
        for f in skips.fields.iter() {
            let s = field_to_skippable(f.id).unwrap();
            let entry = SKIPPABLE_QUESTIONS.iter().find(|e| e.id == s).unwrap();
            // A spouse-gated skippable needs a spouse present for its setter to stick; the §G-9 dates
            // of death additionally need their gate to say the person died (`SkippableId::Dod*` is live
            // only then); the Schedule B 7a FBAR sub-question needs 7a = Yes (the form's own "If
            // 'Yes,'"). Every primer is harmless to the other entries' liveness.
            let seed = |ri: &mut ReturnInputs| {
                if !(entry.live)(ri) {
                    // ★ MFJ first: `SpouseDiedDuringYear` and `DodSpouse` are MFJ-gated (the only
                    // status whose spouse §63(f) box is counted), so a spouse `Person` alone is no
                    // longer enough to make them live.
                    ri.filing_status = FilingStatus::Mfj;
                    ri.header.spouse = Some(Person::default());
                    ri.header.taxpayer_died_during_year = Some(true);
                    ri.header.spouse_died_during_year = Some(true);
                    ri.foreign_accounts = Some(true);
                    // ★ Schedule C lines I/J: a `schedule_c` to write onto, and — for line J — line I
                    // answered YES, since the form asks J only "If 'Yes,'".
                    ri.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
                        payments_requiring_1099: Some(true),
                        ..Default::default()
                    });
                }
                // ★★★ FR-29 — Form 8615's trio needs the OPPOSITE of the MFJ priming above:
                //     condition 5 is "you don't file a joint return", so all three are dead on MFJ.
                //     The dead-end fact additionally needs condition 4 answered CANNOT KNOW, which is
                //     the whole of the owner ruling's first constraint — it is not offered until the
                //     filer has testified to the dead end. So it is primed here, and ONLY for an entry
                //     the MFJ pass above left dead.
                if !(entry.live)(ri) {
                    ri.filing_status = FilingStatus::Single;
                    ri.header.taxpayer.date_of_birth = None;
                    ri.header.form8615_condition3_age_support = Some(true);
                    ri.header.form8615_condition4_parent_alive =
                        Some(ParentAliveAnswer::CannotKnow);
                }
            };
            match entry.kind {
                SkippableKind::YesNo => {
                    let mut ri = fresh_single();
                    seed(&mut ri);
                    (entry.set_bool)(&mut ri, true);
                    assert_eq!(
                        (f.get)(&ri, &RowAddr::default()),
                        Some(FieldValue::TriState(Some(true)))
                    );
                    let mut ri2 = ri.clone();
                    (f.set)(
                        &mut ri2,
                        &RowAddr::default(),
                        FieldValue::TriState(Some(false)),
                    )
                    .unwrap();
                    assert_eq!(
                        (entry.get_bool)(&ri2),
                        Some(false),
                        "set delegation {:?}",
                        f.id
                    );
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
                    assert_eq!(
                        (f.get)(&ri, &RowAddr::default()),
                        Some(FieldValue::Date(Some(d)))
                    );
                    let mut ri2 = ri.clone();
                    let d2 = date!(1985 - 05 - 05);
                    (f.set)(&mut ri2, &RowAddr::default(), FieldValue::Date(Some(d2))).unwrap();
                    assert_eq!(
                        (entry.get_date)(&ri2),
                        Some(d2),
                        "set delegation {:?}",
                        f.id
                    );
                    assert_eq!(
                        (f.set)(&mut ri2, &RowAddr::default(), FieldValue::TriState(None)),
                        Err(SetError::WrongKind)
                    );
                }
                // ★★★ FR-29 — the one `Choice` skippable (Form 8615's condition 4). Its third answer
                //     is not "unanswered", so the round-trip is over the TOKENS, and an unlisted
                //     string must be `WrongKind` rather than a silent `Ok` that stored nothing.
                SkippableKind::Choice(options) => {
                    let mut ri = fresh_single();
                    seed(&mut ri);
                    (entry.set_choice)(&mut ri, options[0]);
                    assert_eq!(
                        (f.get)(&ri, &RowAddr::default()),
                        Some(FieldValue::Choice(options[0].to_string()))
                    );
                    let mut ri2 = ri.clone();
                    (f.set)(
                        &mut ri2,
                        &RowAddr::default(),
                        FieldValue::Choice(options[2].to_string()),
                    )
                    .unwrap();
                    assert_eq!(
                        (entry.get_choice)(&ri2),
                        Some(options[2]),
                        "set delegation {:?}",
                        f.id
                    );
                    assert_eq!(
                        (f.set)(
                            &mut ri2,
                            &RowAddr::default(),
                            FieldValue::Choice("Maybe".to_string())
                        ),
                        Err(SetError::WrongKind),
                        "an unlisted choice must not be stored, and must not report Ok"
                    );
                    assert_eq!(
                        (entry.get_choice)(&ri2),
                        Some(options[2]),
                        "and the rejected set must have changed nothing"
                    );
                    assert_eq!(
                        (f.set)(&mut ri2, &RowAddr::default(), FieldValue::TriState(None)),
                        Err(SetError::WrongKind)
                    );
                }
            }
        }

        // The spouse-gated liveness edge: BlindSpouse is live only when a spouse Person is present.
        let blind_spouse = skips
            .fields
            .iter()
            .find(|f| f.id == FieldId::BlindSpouse)
            .unwrap();
        let mut ri = fresh_single();
        assert!(
            !(blind_spouse.live)(&ri),
            "BlindSpouse is not live without a spouse"
        );
        ri.header.spouse = Some(Person::default());
        assert!(
            (blind_spouse.live)(&ri),
            "BlindSpouse is live with a spouse present"
        );
    }

    /// ★★★ **FR-29 / SPEC §9 G14(a), THE POLARITY ROW — the only test in the repo that reds on a
    /// straight-through accessor pair.**
    ///
    /// It lives here, not beside the rest of G14(a) in `btctax-core`, because it must go in through
    /// the **`Field`** the way the interview does — and `form_spec()` is in this crate, which depends
    /// on `btctax-core` rather than the other way round.
    ///
    /// `TriState(Some(true))` is the filer answering **YES** to *"Can you give the IRS your parent's
    /// name and address?"* — the filer for whom the IRS-request route is OPEN, and for whom R-4 is
    /// meant to be final. The correct outcome is the refusal that sends them to that route.
    ///
    /// **Mutation — and it is the whole point:** write the accessors straight through
    /// (`get_bool: |ri| ri.header.form8615_parent_identity_unobtainable`,
    /// `set_bool: |ri, v| ri.header.form8615_parent_identity_unobtainable = Some(v)`) ⇒ **red**,
    /// because a YES then certifies and the filer receives a computed return with no Form 8615. That
    /// is FR-29 rebuilt inside the fix for FR-29.
    ///
    /// ★★ Nothing else in the repo reds on it. Every other Form 8615 assertion sets the FIELD VALUE
    /// and so never sees the keystroke; and
    /// `skippables_section_delegates_every_non_salt_skippable_and_the_map_is_total` round-trips
    /// `set` → `get` through the pair, which passes on a consistently-inverted pair AND on a
    /// straight-through one — it detects a get/set MISMATCH, which is not this defect.
    #[test]
    fn the_dead_end_accessors_invert_so_a_yes_is_still_refused() {
        use btctax_core::state::LedgerState;
        use btctax_core::tax::return_1040::screen_compute_dependent;
        use btctax_core::tax::return_inputs::{Form1099Int, ParentAliveAnswer};
        use btctax_core::tax::return_refuse::RefuseReason;
        use rust_decimal_macros::dec;

        const YEAR: i32 = 2024;
        use btctax_core::tax::tables::FullReturnTables as _;
        let tables = btctax_adapters::BundledFullReturnTables::load();
        let params = tables
            .full_return_for(YEAR)
            .expect("TY2024 is a supported full-return year");

        // A filer inside Form 8615's reach, in the dead end: over the §1(g) threshold, not joint, no
        // date of birth, condition 3 = YES, condition 4 = CANNOT KNOW.
        let base = || {
            let mut ri = fresh_single();
            ri.tax_year = YEAR;
            ri.header.form8615_condition3_age_support = Some(true);
            ri.header.form8615_condition4_parent_alive = Some(ParentAliveAnswer::CannotKnow);
            ri.int_1099 = vec![Form1099Int {
                box1_interest: dec!(9000),
                ..Default::default()
            }];
            ri
        };
        let f = form_spec()
            .iter()
            .flat_map(|s| s.fields.iter())
            .find(|f| f.id == FieldId::Form8615ParentIdentityUnobtainable)
            .expect("the dead-end fact has a Field");
        let state = LedgerState::default();

        // The filer's YES — "I CAN supply a name and address" — must still REFUSE.
        let mut yes = base();
        (f.set)(
            &mut yes,
            &RowAddr::default(),
            FieldValue::TriState(Some(true)),
        )
        .expect("the question is live in the dead end");
        assert_eq!(
            screen_compute_dependent(&yes, &state, YEAR, params).map(|r| r.reason),
            Some(RefuseReason::Form8615ParentUnidentifiable),
            "THE POLARITY. A straight-through accessor pair stores the filer's YES as \
             `unobtainable = Some(true)`, certifies them, and hands them a computed return with no \
             Form 8615 — an understatement path, and the very escape FR-29 was."
        );
        assert_eq!(
            yes.header.form8615_parent_identity_unobtainable,
            Some(false),
            "the filer's YES is the leaf's `Some(false)` — the accessors invert"
        );

        // The filer's NO — "I can supply NEITHER" — is the one that certifies.
        let mut no = base();
        (f.set)(
            &mut no,
            &RowAddr::default(),
            FieldValue::TriState(Some(false)),
        )
        .expect("the question is live in the dead end");
        assert_eq!(
            screen_compute_dependent(&no, &state, YEAR, params).map(|r| r.reason),
            None,
            "and the filer's NO is what opens the §6.3 certification"
        );
        assert_eq!(
            no.header.form8615_parent_identity_unobtainable,
            Some(true),
            "the filer's NO is the leaf's `Some(true)`"
        );

        // …and `get` shows the filer their OWN answer to the prompt, not the leaf.
        assert_eq!(
            (f.get)(&no, &RowAddr::default()),
            Some(FieldValue::TriState(Some(false))),
            "get must round-trip the ANSWER, so the interview redisplays what the filer said"
        );
        assert_eq!(
            (f.get)(&yes, &RowAddr::default()),
            Some(FieldValue::TriState(Some(true)))
        );
    }
}
