//! ★ The edit-application layer (spec §5.7 / §10) — turns a stream of [`Edit`]s into mutations of a working
//! [`ReturnInputs`], with the anti-laundering NI-2 invariant at its core: a return cannot exist until its
//! filing status is explicitly chosen. `Working = Option<ReturnInputs>`; `None` means "no return yet". The
//! FIRST accepted edit MUST be `SetField{FilingStatus, Choice(_)}`, which materializes the return; any other
//! edit on `None` is refused and materializes nothing. This makes "filing status chosen ≡ a `ReturnInputs`
//! exists" a type-level fact, so a later `commit` can only ever see a return whose `Single` was *chosen*,
//! never a laundered `ReturnInputs::default()`.

use crate::seam::{
    ApplyError, Edit, Field, FieldId, FieldKind, FieldValue, RowAddr, Section, SectionId,
    SectionKind, SetError,
};
use crate::spec::form_spec;
use btctax_core::tax::provenance::{
    current_prompt, forget_answer, record_answer, AnswerKey, AnswerState,
};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::Usd;
use time::Date;

/// The working return under edit. `None` = filing status not yet chosen (no return materialized).
pub type Working = Option<ReturnInputs>;

/// ★ Apply one edit to the working return (spec §5.7 / §10).
///
/// **NI-2 materialization (the anti-laundering guard):** when `*w` is `None`, the ONLY accepted edit is
/// `SetField{ id: FilingStatus, value: Choice(_) }`, which materializes a fresh return whose filing status is
/// the chosen one and every other field is default. ANY other edit on `None` is refused ([`WrongFirstEdit`])
/// and materializes nothing — a return cannot exist until its status is explicitly chosen.
///
/// [`WrongFirstEdit`]: ApplyError::WrongFirstEdit
/// ★★★ **R10.3 — the answer-log key a `FieldId` writes to, or `None` for a plain leaf.**
///
/// Derived from the two registry maps that already exist ([`crate::spec::field_to_question`] /
/// [`crate::spec::field_to_skippable`]), so the set of fields that RECORD is the set of fields that
/// delegate to a registry question — by identity, not by a second hand-written list that can drift.
fn answer_key_for(id: FieldId) -> Option<AnswerKey> {
    if let Some(q) = crate::spec::field_to_question(id) {
        return Some(AnswerKey::Question(q));
    }
    crate::spec::field_to_skippable(id).map(AnswerKey::Skippable)
}

pub fn apply(w: &mut Working, e: Edit, now: Date) -> Result<(), ApplyError> {
    match w {
        // ★ NI-2: nothing exists yet — only the filing-status *choice* brings a return into being.
        None => match e {
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr,
                value: value @ FieldValue::Choice(_),
            } => {
                let (field, depth) =
                    locate_field(FieldId::FilingStatus).ok_or(ApplyError::NoSuchSection)?;
                // ★ (m): reject a malformed-arity addr on the FIRST edit too — parity with `apply_to`, which
                // guards. Without this, an over-long addr was accepted here but rejected post-materialization.
                guard_arity(&addr, depth)?;
                // Set the status on an otherwise-pure default; only assign `*w` on success, so a bad choice
                // (e.g. an unknown status string) leaves `*w` as `None` — nothing laundered.
                let mut ri = ReturnInputs::default();
                (field.set)(&mut ri, &addr, value).map_err(ApplyError::SetError)?;
                *w = Some(ri);
                Ok(())
            }
            _ => Err(ApplyError::WrongFirstEdit),
        },
        Some(ri) => apply_to(ri, e, now),
    }
}

/// Dispatch an edit against a materialized return.
fn apply_to(ri: &mut ReturnInputs, e: Edit, now: Date) -> Result<(), ApplyError> {
    match e {
        Edit::SetField { id, addr, value } => {
            let (field, depth) = locate_field(id).ok_or(ApplyError::NoSuchSection)?;
            guard_arity(&addr, depth)?;
            (field.set)(ri, &addr, value).map_err(ApplyError::SetError)?;
            // ★★★ R10.3 — THE ONE WRITER, reached from the editor. `income answer` reaches the same
            //     function with the same `now`, which is why the two surfaces produce byte-identical
            //     records for the same answer. Recorded only AFTER the set succeeds: a refused edit
            //     changed nothing, so it is not an answer.
            if let Some(key) = answer_key_for(id) {
                // ★ R10.4 — the words are RENDERED FROM THE RETURN (`prompt_text`), so a question
                //   that quotes a value hashes the sentence the filer actually saw. Owned first,
                //   because `record_answer` takes `&mut ri` and the rendered text borrows it.
                let prompt = current_prompt(&key, ri).map(std::borrow::Cow::into_owned);
                if let Some(prompt) = prompt {
                    record_answer(ri, key, &prompt, now, AnswerState::Given);
                }
            }
            Ok(())
        }
        Edit::ClearField { id, addr } => {
            let (field, depth) = locate_field(id).ok_or(ApplyError::NoSuchSection)?;
            guard_arity(&addr, depth)?;
            // ★ The un-answer path (spec §5.7 M-6, review I-1). An `Enum` has no empty state (this includes
            // `filing_status`) → `Immutable`, WITHOUT touching anything. Else, a field carrying a dedicated
            // `clear` (the registry-delegating tri-state/date leaves — whose registry setter writes only a
            // definite yes/no, so it cannot un-answer) routes through it, writing its `Option` leaf to `None`.
            // Every plain field clears via its own `set(empty_for_kind)`.
            if let FieldKind::Enum(_) = field.kind {
                return Err(ApplyError::SetError(SetError::Immutable));
            }
            // ★ R10.3 — un-answering returns the question to NEVER ASKED, so the record goes. It is
            //   NOT recorded as `Declined` (that is *"asked and passed over"*, which is a different
            //   act, and a class-(A) declaration has no lawful decline at all) and it writes no
            //   history (§5.6 says exactly what history holds, and a clear is neither).
            if let Some(clear) = field.clear {
                clear(ri, &addr).map_err(ApplyError::SetError)?;
                if let Some(key) = answer_key_for(id) {
                    forget_answer(ri, &key);
                }
                return Ok(());
            }
            let empty = match field.kind {
                FieldKind::Money => FieldValue::Money(Usd::ZERO),
                FieldKind::Text => FieldValue::Text(String::new()),
                FieldKind::Bool => FieldValue::Bool(false),
                FieldKind::Date => FieldValue::Date(None),
                FieldKind::TriState => FieldValue::TriState(None),
                FieldKind::Secret => FieldValue::SecretEntry(String::new()),
                FieldKind::Enum(_) => unreachable!("Enum returned Immutable above"),
            };
            (field.set)(ri, &addr, empty).map_err(ApplyError::SetError)?;
            if let Some(key) = answer_key_for(id) {
                forget_answer(ri, &key);
            }
            Ok(())
        }
        Edit::AddRow { section, parent } => {
            let s = find_section(section).ok_or(ApplyError::NoSuchSection)?;
            let SectionKind::Repeating { add, .. } = s.kind else {
                return Err(ApplyError::NoSuchSection);
            };
            // `parent` addresses the CONTAINER — one level shallower than a row of this section.
            guard_arity(&parent, row_depth(section).saturating_sub(1))?;
            // ★ I-4: propagate the builder's `Result` — an absent parent is `NoSuchRow`, not a lying `Ok`.
            add(ri, &parent).map_err(ApplyError::SetError)
        }
        Edit::RemoveRow { section, addr } => {
            let s = find_section(section).ok_or(ApplyError::NoSuchSection)?;
            let SectionKind::Repeating { remove, .. } = s.kind else {
                return Err(ApplyError::NoSuchSection);
            };
            guard_arity(&addr, row_depth(section))?;
            remove(ri, &addr).map_err(ApplyError::SetError)
        }
        Edit::CreateSection { section } => {
            let s = find_section(section).ok_or(ApplyError::NoSuchSection)?;
            let SectionKind::OptionalSingleton { create, .. } = s.kind else {
                return Err(ApplyError::NoSuchSection);
            };
            create(ri);
            Ok(())
        }
        Edit::DeleteSection { section } => {
            let s = find_section(section).ok_or(ApplyError::NoSuchSection)?;
            let SectionKind::OptionalSingleton { delete, .. } = s.kind else {
                return Err(ApplyError::NoSuchSection);
            };
            // ScheduleA's `delete` also performs the I-10 `itemize_election → Auto` reset (sections.rs) — we
            // route through it rather than re-implement it.
            delete(ri);
            Ok(())
        }
    }
}

/// The `RowAddr` depth a section requires to name a row: singletons/optional-singletons `0`, the depth-1
/// repeating groups (Dependents/W2s/ScheduleACharitable) `1`, the nested box-12 group `2`. Exhaustive so a
/// new `SectionId` is a compile error here.
fn row_depth(id: SectionId) -> usize {
    match id {
        SectionId::Dependents
        | SectionId::W2s
        | SectionId::ScheduleACharitable
        | SectionId::BrokerReporting => 1,
        SectionId::W2Box12 => 2,
        SectionId::ReturnOptions
        | SectionId::Taxpayer
        | SectionId::Spouse
        | SectionId::Address
        | SectionId::ScheduleA
        | SectionId::Payments
        | SectionId::Carryforwards
        | SectionId::QbiLimitation
        | SectionId::Declarations
        // ★ R3 — the census is a SINGLETON: one tri-state per document TYPE, not per document. The
        //   per-document rows live in the document sections (`W2s`, and T5's 1099 sections).
        | SectionId::DocumentCensus
        | SectionId::IncomeExclusions
        | SectionId::Skippables => 0,
    }
}

/// ★ Fail-closed arity guard (untrusted wire input, spec §4/§13): the row accessors index `a.0[0]`/`a.0[1]`
/// and PANIC on a short vector, so refuse a too-shallow addr BEFORE any accessor sees it. EXACT arity per
/// depth (review M-2): a too-LONG addr (extra indices a section never reads) is refused too — a fail-closed
/// contract, not silently-ignored trailing junk.
fn guard_arity(addr: &RowAddr, required: usize) -> Result<(), ApplyError> {
    if addr.0.len() == required {
        Ok(())
    } else {
        Err(ApplyError::SetError(SetError::NoSuchRow))
    }
}

/// Locate a field by id across the spec, returning it with the `RowAddr` depth its owning section requires.
fn locate_field(id: FieldId) -> Option<(&'static Field, usize)> {
    for s in form_spec() {
        if let Some(f) = s.fields.iter().find(|f| f.id == id) {
            return Some((f, row_depth(s.id)));
        }
    }
    None
}

/// Locate a section by its stable id.
fn find_section(id: SectionId) -> Option<&'static Section> {
    form_spec().iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::{apply, Working};
    use crate::seam::{ApplyError, Edit, FieldId, FieldValue, RowAddr, SectionId, SetError};
    use btctax_core::tax::return_inputs::{ItemizeElection, ReturnInputs};
    use btctax_core::tax::types::FilingStatus;
    use rust_decimal_macros::dec;
    use time::macros::date;

    /// Materialize a working return by choosing `fs` — the ONLY way a return comes into being (NI-2).
    fn materialize(w: &mut Working, fs: FilingStatus) {
        apply(
            w,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice(fs_name(fs).into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
    }

    fn fs_name(fs: FilingStatus) -> &'static str {
        match fs {
            FilingStatus::Single => "Single",
            FilingStatus::Mfj => "Mfj",
            FilingStatus::Mfs => "Mfs",
            FilingStatus::HoH => "HoH",
            FilingStatus::Qss => "Qss",
        }
    }

    /// (m): a malformed-arity addr must be rejected on the FIRST (materializing) edit too — parity with
    /// `apply_to`. FilingStatus is depth-0, so ANY index is malformed; a depth-0 `set` would otherwise ignore
    /// the junk addr and materialize, so this kills the missing-guard mutant. The rejection materializes
    /// nothing.
    #[test]
    fn m_first_edit_rejects_a_malformed_arity_addr() {
        let mut w: Working = None;
        let r = apply(
            &mut w,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr(vec![0]),
                value: FieldValue::Choice("Single".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        );
        assert!(
            matches!(r, Err(ApplyError::SetError(SetError::NoSuchRow))),
            "(m): first-edit malformed arity must be rejected; got {r:?}"
        );
        assert!(
            w.is_none(),
            "(m): a rejected first edit materializes nothing"
        );
    }

    /// The brief's Step-1 test: a fresh working accepts only the filing-status choice first, then materializes.
    #[test]
    fn fresh_working_only_accepts_filing_status_first_then_materializes() {
        let mut w: Working = None;
        // a non-filing-status edit is rejected, leaving None
        let bad = apply(
            &mut w,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(1)),
            },
            time::macros::date!(2026 - 09 - 01),
        );
        assert_eq!(bad, Err(ApplyError::WrongFirstEdit));
        assert!(w.is_none());
        // choosing filing status materializes exactly that, all else default
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Mfj".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        let ri = w.as_ref().unwrap();
        assert_eq!(ri.filing_status, FilingStatus::Mfj);
        assert_eq!(ri.w2s.len(), 0);
        // filing_status can never be cleared (Enum, no empty state)
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::FilingStatus,
                    addr: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::Immutable))
        );
    }

    /// ★ NI-2 (spec §10 / M-3): on `None`, every edit but the filing-status choice is refused and materializes
    /// nothing; the choice materializes EXACTLY that status over an otherwise-pure default; the status never
    /// returns to `None`.
    #[test]
    fn ni2_none_rejects_all_but_filing_status_then_materializes_pure_default() {
        let rejects = [
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(5)),
            },
            Edit::ClearField {
                id: FieldId::TpFirstName,
                addr: RowAddr::default(),
            },
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            Edit::CreateSection {
                section: SectionId::Spouse,
            },
            Edit::DeleteSection {
                section: SectionId::ScheduleA,
            },
            // A filing-status edit whose value is NOT a Choice is not the accepted shape either.
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Money(dec!(1)),
            },
        ];
        for e in rejects {
            let mut w: Working = None;
            assert_eq!(
                apply(&mut w, e.clone(), time::macros::date!(2026 - 09 - 01)),
                Err(ApplyError::WrongFirstEdit),
                "must refuse on None: {e:?}"
            );
            assert!(
                w.is_none(),
                "nothing may materialize on a refused first edit: {e:?}"
            );
        }
        // The choice materializes exactly that status over an otherwise-pure default.
        for (name, fs) in [
            ("Single", FilingStatus::Single),
            ("Mfj", FilingStatus::Mfj),
            ("Mfs", FilingStatus::Mfs),
            ("HoH", FilingStatus::HoH),
            ("Qss", FilingStatus::Qss),
        ] {
            let mut w: Working = None;
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::FilingStatus,
                    addr: RowAddr::default(),
                    value: FieldValue::Choice(name.into()),
                },
                time::macros::date!(2026 - 09 - 01),
            )
            .unwrap();
            let expected = ReturnInputs {
                filing_status: fs,
                ..Default::default()
            };
            assert_eq!(
                w.as_ref().unwrap(),
                &expected,
                "{name}: pure default + that status only"
            );
            assert!(w.as_ref().unwrap().w2s.is_empty());
            assert!(w.as_ref().unwrap().schedule_a.is_none());
        }
    }

    /// ★ NI-2 edge: a correctly-shaped filing-status choice with an UNPARSEABLE status string must not launder
    /// a return — the setter rejects it and `*w` stays `None` (we only assign on a successful set).
    #[test]
    fn ni2_bad_filing_status_choice_leaves_none_nothing_laundered() {
        let mut w: Working = None;
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::FilingStatus,
                    addr: RowAddr::default(),
                    value: FieldValue::Choice("Nope".into()),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::WrongKind)),
        );
        assert!(
            w.is_none(),
            "no return may materialize from an unparseable filing-status choice"
        );
    }

    /// ★★ **N1 — a census row's THREE arms all check liveness, `clear` included** (T3 seam review).
    ///
    /// `get` and `set` returned early on a non-live row from the start; `clear` wrote through. It is
    /// harmless at HEAD (the two non-live rows are already `None`, so the write is a no-op), which
    /// is exactly why it needed a test rather than an argument: the moment `row_is_live` gains a
    /// real predicate — T9's `schedule_a.is_some()` for `form_1098` — an unguarded `clear` is a
    /// write to a row the filer is not being asked, through a section that is not being shown.
    #[test]
    fn clearing_a_non_live_census_row_is_refused_exactly_as_setting_it_is() {
        let now = time::macros::date!(2026 - 09 - 01);
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);

        // `form_1098` is scalar-shadowed and therefore not live (T9 opens it).
        for (id, live) in [(FieldId::DocForm1098, false), (FieldId::DocW2, true)] {
            let set = apply(
                &mut w,
                Edit::SetField {
                    id,
                    addr: RowAddr::default(),
                    value: FieldValue::TriState(Some(true)),
                },
                now,
            );
            let cleared = apply(
                &mut w,
                Edit::ClearField {
                    id,
                    addr: RowAddr::default(),
                },
                now,
            );
            if live {
                set.expect("a live census row is settable");
                cleared.expect("…and clearable");
            } else {
                assert_eq!(
                    set,
                    Err(ApplyError::SetError(SetError::NoSuchRow)),
                    "{id:?} is not live: `set` must refuse"
                );
                assert_eq!(
                    cleared,
                    Err(ApplyError::SetError(SetError::NoSuchRow)),
                    "★ …and `clear` must refuse on the SAME predicate — un-answering a row nobody \
                     is being asked is still a write to it"
                );
            }
        }
    }

    /// ★★★ **R3 — a census `No` over transcribed rows is REFUSED, not stored and not silently
    ///     destructive** (the `DeleteSection(ScheduleA)` I-10 precedent).
    ///
    /// Three states, and all three matter:
    ///   1. `No` with a W-2 on the return ⇒ `ContradictsTranscribedRows { rows: 1 }`, and the leaf is
    ///      UNCHANGED — a store would leave the return in `DocumentCensusContradicted` with no
    ///      in-form remedy, and a delete would destroy transcribed testimony on one keystroke;
    ///   2. `Yes` is always accepted — a yes contradicts nothing;
    ///   3. `No` once the rows are gone is accepted — the refusal is a guard, not a one-way door.
    #[test]
    fn a_census_no_over_transcribed_rows_is_refused_and_stores_nothing() {
        let now = time::macros::date!(2026 - 09 - 01);
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            now,
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s.len(), 1);

        let set_w2_census = |w: &mut Working, v: bool| {
            apply(
                w,
                Edit::SetField {
                    id: FieldId::DocW2,
                    addr: RowAddr::default(),
                    value: FieldValue::TriState(Some(v)),
                },
                now,
            )
        };

        // (1) "No" over a transcribed row: refused, and NOTHING is written.
        assert_eq!(
            set_w2_census(&mut w, false),
            Err(ApplyError::SetError(SetError::ContradictsTranscribedRows {
                rows: 1
            })),
            "a census No beside a transcribed W-2 must be refused with the count"
        );
        assert_eq!(
            w.as_ref().unwrap().documents.w2,
            None,
            "the refused write must store nothing — a stored No is the contradicted state"
        );
        assert_eq!(
            w.as_ref().unwrap().w2s.len(),
            1,
            "★ and it must NOT delete the row to make the answer true"
        );

        // (2) "Yes" is accepted — the guard is on No alone.
        set_w2_census(&mut w, true).unwrap();
        assert_eq!(w.as_ref().unwrap().documents.w2, Some(true));

        // (3) Remove the row, and "No" is then accepted — a guard, not a one-way door.
        apply(
            &mut w,
            Edit::RemoveRow {
                section: SectionId::W2s,
                addr: RowAddr(vec![0]),
            },
            now,
        )
        .unwrap();
        set_w2_census(&mut w, false).unwrap();
        assert_eq!(w.as_ref().unwrap().documents.w2, Some(false));

        // ── ★★★ D1 — THE SAME GUARD ON ALL FOUR 1099 ROWS. ──────────────────────────────────────
        //
        // They have no `AddRow` section until T5, so the row is put on the return the way `income
        // import` puts it there: straight onto the `Vec`. The guard must not care which writer
        // filled it — the contradiction is between the ANSWER and the DATA.
        use btctax_core::tax::document_census::DocumentRow;
        use btctax_core::tax::return_inputs::{Form1099B, Form1099Div, Form1099G, Form1099Int};
        for (row, fid) in [
            (DocumentRow::Int1099, FieldId::DocInt1099),
            (DocumentRow::Div1099, FieldId::DocDiv1099),
            (DocumentRow::B1099, FieldId::DocB1099),
            (DocumentRow::G1099, FieldId::DocG1099),
        ] {
            let mut w: Working = None;
            materialize(&mut w, FilingStatus::Single);
            {
                let ri = w.as_mut().unwrap();
                match row {
                    DocumentRow::Int1099 => ri.int_1099 = vec![Form1099Int::default()],
                    DocumentRow::Div1099 => ri.div_1099 = vec![Form1099Div::default()],
                    DocumentRow::B1099 => ri.b_1099 = vec![Form1099B::default()],
                    DocumentRow::G1099 => ri.g_1099 = vec![Form1099G::default()],
                    other => panic!("{other:?} is not one of the four"),
                }
            }
            let set = |w: &mut Working, v: bool| {
                apply(
                    w,
                    Edit::SetField {
                        id: fid,
                        addr: RowAddr::default(),
                        value: FieldValue::TriState(Some(v)),
                    },
                    now,
                )
            };
            assert_eq!(
                set(&mut w, false),
                Err(ApplyError::SetError(SetError::ContradictsTranscribedRows {
                    rows: 1
                })),
                "{row:?}: a census No beside a transcribed row must be refused with the count"
            );
            assert_eq!(
                w.as_ref().unwrap().documents.get(row),
                None,
                "{row:?}: the refused write must store nothing"
            );
            set(&mut w, true).unwrap();
            assert_eq!(w.as_ref().unwrap().documents.get(row), Some(true));
        }
    }

    /// I-10 (spec §10): a `ForceItemize` + `DeleteSection(ScheduleA)` leaves `itemize_election == Auto` — a
    /// return with no Schedule A can never keep forcing itemization (routed through `sections.rs`'s delete).
    #[test]
    fn delete_schedule_a_resets_forced_itemize_i10() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::ItemizeElection,
                addr: RowAddr::default(),
                value: FieldValue::Choice("ForceItemize".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().itemize_election,
            ItemizeElection::ForceItemize
        );
        assert!(w.as_ref().unwrap().schedule_a.is_some());

        apply(
            &mut w,
            Edit::DeleteSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().itemize_election,
            ItemizeElection::Auto,
            "I-10 reset"
        );
        assert!(w.as_ref().unwrap().schedule_a.is_none());
    }

    /// Tree edits: AddRow/RemoveRow including the nested box-12 at depth 2, and Create/DeleteSection for the two
    /// optional singletons (Spouse, Schedule A).
    #[test]
    fn tree_edits_add_remove_rows_and_sections_incl_box12_depth2() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);

        // W2 row (depth 1).
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s.len(), 1);
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(50000)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box1_wages, dec!(50000));

        // Nested box-12 row (depth 2), parent = [0].
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2Box12,
                parent: RowAddr(vec![0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box12.len(), 1);
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::Box12Amount,
                addr: RowAddr(vec![0, 0]),
                value: FieldValue::Money(dec!(23000)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box12[0].amount, dec!(23000));

        // RemoveRow box-12 at [0,0], then the W2 at [0].
        apply(
            &mut w,
            Edit::RemoveRow {
                section: SectionId::W2Box12,
                addr: RowAddr(vec![0, 0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().w2s[0].box12.is_empty());
        apply(
            &mut w,
            Edit::RemoveRow {
                section: SectionId::W2s,
                addr: RowAddr(vec![0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().w2s.is_empty());

        // Spouse optional-singleton create → set → delete.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::Spouse,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().header.spouse.is_some());
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::SpFirstName,
                addr: RowAddr::default(),
                value: FieldValue::Text("Pat".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref()
                .unwrap()
                .header
                .spouse
                .as_ref()
                .unwrap()
                .first_name,
            "Pat"
        );
        apply(
            &mut w,
            Edit::DeleteSection {
                section: SectionId::Spouse,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().header.spouse.is_none());

        // Schedule A optional-singleton create → delete.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().schedule_a.is_some());
        apply(
            &mut w,
            Edit::DeleteSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(w.as_ref().unwrap().schedule_a.is_none());
    }

    /// ★ Fail-closed on malformed `RowAddr` arity (untrusted wire input): a short/empty addr must be a clean
    /// error, NEVER a panic in a row accessor that indexes `a.0[0]`/`a.0[1]`.
    #[test]
    fn malformed_short_rowaddr_is_rejected_not_panicked() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);

        // A W2 money leaf needs depth 1; an empty addr is a clean error, not a panic.
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::Box1Wages,
                    addr: RowAddr(vec![]),
                    value: FieldValue::Money(dec!(1)),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // A box-12 leaf needs depth 2; a depth-1 addr is too short.
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::Box12Amount,
                    addr: RowAddr(vec![0]),
                    value: FieldValue::Money(dec!(1)),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // AddRow box-12 with an EMPTY parent would panic in the accessor (`a.0[0]`); the guard prevents it.
        assert_eq!(
            apply(
                &mut w,
                Edit::AddRow {
                    section: SectionId::W2Box12,
                    parent: RowAddr(vec![])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // RemoveRow box-12 with a depth-1 addr is too short.
        assert_eq!(
            apply(
                &mut w,
                Edit::RemoveRow {
                    section: SectionId::W2Box12,
                    addr: RowAddr(vec![0])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // ClearField on a W2 leaf with a short addr is likewise a clean error.
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::Box1Wages,
                    addr: RowAddr(vec![])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
    }

    /// ★★★ §G-28/B1b — CLEARING AN `Option<Usd>` MONEY FIELD MUST WRITE `None`, NOT `Some($0)`.
    ///
    /// `qbi_w2_wages` and `qbi_ubia` are the ONLY two `Option<Usd>` leaves in `ReturnInputs`, so they
    /// are the only fields where the generic un-answer path would launder "un-answered" into the
    /// ANSWER "my business paid no wages". Those are not the same testimony, and the difference is
    /// the whole of `screen_absolute`'s `QbiAboveThreshold` guard: it refuses on `is_none()`, so a
    /// laundered `Some(0)` walks straight past it into `unwrap_or(Usd::ZERO)` and caps a
    /// wage-paying filer's §199A deduction at zero — OVERSTATING their tax.
    ///
    /// ★ This is the case the kind-matrix below did not have. It reds if either field's dedicated
    ///   `clear` is dropped back to `None`.
    #[test]
    fn clearing_an_option_money_field_un_answers_it_rather_than_answering_zero() {
        for (id, get) in [
            (
                FieldId::QbiW2Wages,
                (|ri: &ReturnInputs| ri.schedule_c.as_ref().and_then(|c| c.qbi_w2_wages))
                    as fn(&ReturnInputs) -> Option<btctax_core::Usd>,
            ),
            (FieldId::QbiUbia, |ri: &ReturnInputs| {
                ri.schedule_c.as_ref().and_then(|c| c.qbi_ubia)
            }),
        ] {
            let mut w: Working = None;
            materialize(&mut w, FilingStatus::Single);
            w.as_mut().unwrap().schedule_c = Some(Default::default());

            // Answer it with a real figure…
            apply(
                &mut w,
                Edit::SetField {
                    id,
                    addr: RowAddr::default(),
                    value: FieldValue::Money(rust_decimal_macros::dec!(120000)),
                },
                time::macros::date!(2026 - 09 - 01),
            )
            .expect("set");
            assert_eq!(
                get(w.as_ref().unwrap()),
                Some(rust_decimal_macros::dec!(120000)),
                "{id:?}: the fixture must actually be answered first"
            );

            // …then un-answer it, and it must be BLANK, not zero.
            apply(
                &mut w,
                Edit::ClearField {
                    id,
                    addr: RowAddr::default(),
                },
                time::macros::date!(2026 - 09 - 01),
            )
            .expect("clear");
            assert_eq!(
                get(w.as_ref().unwrap()),
                None,
                "{id:?}: clearing must UN-ANSWER. `Some($0)` is the answer \"no wages\", which \
                 walks past the QbiAboveThreshold refusal that exists to demand this figure"
            );
        }
    }

    /// ★ ClearField per-kind (review I-1/I-2, updated from the old "registry limitation" that pinned the bug):
    /// Enum → `Immutable`; the registry-delegating tri-state/date fields UN-ANSWER their underlying `Option`
    /// leaf to `None` (spec §5.7 M-6); the `IpPin` Secret clears to `None` (never `Some("")`); and everything
    /// else clears to its empty value.
    ///
    /// ★★ **A dedicated `clear` is NOT limited to registry-delegating fields**, and this comment used to
    /// say it was. `QbiW2Wages`/`QbiUbia` are plain `FieldKind::Money` over `Option<Usd>` leaves — the only
    /// two in `ReturnInputs` — so "clear to the empty value" would write `Some($0)`, which is the ANSWER
    /// "my business paid no wages" rather than an un-answer. The rule is about the LEAF's shape, not the
    /// field's registry membership: any field whose leaf is an `Option` needs its own `clear`.
    #[test]
    fn clearfield_kind_matrix_and_registry_unanswer() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);

        // Enum → Immutable (filing_status can never be un-answered).
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::FilingStatus,
                    addr: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::Immutable)),
        );

        // ★ I-1: a registry-delegating TriState (DeclForeignAccounts) un-answers to `None`. Seed a definite
        // answer, then clear, and assert the underlying `Option<bool>` leaf is back to `None`.
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::DeclForeignAccounts,
                addr: RowAddr::default(),
                value: FieldValue::TriState(Some(true)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().foreign_accounts, Some(true));
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::DeclForeignAccounts,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().foreign_accounts,
            None,
            "I-1: TriState un-answers to None"
        );

        // ★ I-1: a registry-delegating Date (DobTaxpayer) un-answers to `None` likewise.
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::DobTaxpayer,
                addr: RowAddr::default(),
                value: FieldValue::Date(Some(date!(1980 - 03 - 04))),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.taxpayer.date_of_birth,
            Some(date!(1980 - 03 - 04))
        );
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::DobTaxpayer,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.taxpayer.date_of_birth,
            None,
            "I-1: Date un-answers to None"
        );

        // ★ I-2: the IpPin Secret (an `Option<String>`) clears to `None`, NOT `Some("")` (the export-brick).
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::IpPin,
                addr: RowAddr::default(),
                value: FieldValue::SecretEntry("112233".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().header.ip_pin.as_deref(), Some("112233"));
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::IpPin,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.ip_pin,
            None,
            "I-2: IpPin clears to None, not Some(\"\")"
        );

        // A PLAIN Date leaf (DepDob) DOES clear to None cleanly.
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::Dependents,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::DepDob,
                addr: RowAddr(vec![0]),
                value: FieldValue::Date(Some(date!(2015 - 06 - 01))),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.dependents[0].date_of_birth,
            Some(date!(2015 - 06 - 01))
        );
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::DepDob,
                addr: RowAddr(vec![0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().header.dependents[0].date_of_birth, None);

        // Plain Money clears to $0.
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(500)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box1_wages, dec!(0));

        // Plain Text clears to "".
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::TpFirstName,
                addr: RowAddr::default(),
                value: FieldValue::Text("Sam".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpFirstName,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().header.taxpayer.first_name, "");

        // Bool clears to false.
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::TpPresidentialFund,
                addr: RowAddr::default(),
                value: FieldValue::Bool(true),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpPresidentialFund,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert!(!w.as_ref().unwrap().header.presidential_fund_taxpayer);

        // Secret clears to empty.
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::TpSsn,
                addr: RowAddr::default(),
                value: FieldValue::SecretEntry("123456789".into()),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpSsn,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().header.taxpayer.ssn, "");
    }

    /// A `SetField`/`ClearField` whose `id` isn't owned by any section is a clean error, never a panic. (Every
    /// real `FieldId` is in the spec by the coverage KAT, so this only guards the defensive path.)
    #[test]
    fn unknown_target_is_a_clean_error() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        // An AddRow on a non-repeating section is a clean refusal.
        assert_eq!(
            apply(
                &mut w,
                Edit::AddRow {
                    section: SectionId::Taxpayer,
                    parent: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::NoSuchSection),
        );
        // CreateSection on a non-optional section is a clean refusal.
        assert_eq!(
            apply(
                &mut w,
                Edit::CreateSection {
                    section: SectionId::Payments
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::NoSuchSection),
        );
    }

    /// ★ I-1: the two Schedule-A registry-delegating tri-states and a skippable tri-state un-answer their
    /// underlying `Option` leaf to `None` when the parent IS present (the parent-absent case is I-4 below).
    #[test]
    fn delegating_clear_unanswers_schedule_a_and_blind_taxpayer_i1() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        // Prime mortgage interest so SaMortgageAllUsed is live (its set/clear gate on `mortgage_question_live`).
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::SaMortgage1098,
                addr: RowAddr::default(),
                value: FieldValue::Money(dec!(1000)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        for id in [FieldId::SaSaltUseSalesTax, FieldId::SaMortgageAllUsed] {
            apply(
                &mut w,
                Edit::SetField {
                    id,
                    addr: RowAddr::default(),
                    value: FieldValue::TriState(Some(true)),
                },
                time::macros::date!(2026 - 09 - 01),
            )
            .unwrap();
            apply(
                &mut w,
                Edit::ClearField {
                    id,
                    addr: RowAddr::default(),
                },
                time::macros::date!(2026 - 09 - 01),
            )
            .unwrap();
        }
        let sa = w.as_ref().unwrap().schedule_a.as_ref().unwrap();
        assert_eq!(
            sa.salt_use_sales_tax, None,
            "I-1: SaSaltUseSalesTax un-answers to None"
        );
        assert_eq!(
            sa.mortgage_all_used_to_buy_build_improve, None,
            "I-1: SaMortgageAllUsed un-answers to None"
        );

        // A skippable tri-state (BlindTaxpayer, always live) likewise.
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::BlindTaxpayer,
                addr: RowAddr::default(),
                value: FieldValue::TriState(Some(true)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::BlindTaxpayer,
                addr: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.taxpayer.blind,
            None,
            "I-1: BlindTaxpayer un-answers to None"
        );
    }

    /// ★ I-4: a presence-gated delegating `SetField` on an absent parent must REFUSE (`NoSuchRow`), never
    /// report a lying `Ok` after silently dropping the write; and a `ClearField` on an absent parent likewise
    /// refuses. When the parent IS present, both succeed.
    #[test]
    fn delegating_set_and_clear_refuse_on_absent_parent_i4() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single); // no spouse, no schedule_a

        // BlindSpouse with no spouse → NoSuchRow (was a silent Ok that dropped the §63(f) claim).
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::BlindSpouse,
                    addr: RowAddr::default(),
                    value: FieldValue::TriState(Some(true)),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // SaSaltUseSalesTax with no schedule_a → NoSuchRow.
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::SaSaltUseSalesTax,
                    addr: RowAddr::default(),
                    value: FieldValue::TriState(Some(true)),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // A clear on the same absent parents also refuses (not a silent Ok).
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::BlindSpouse,
                    addr: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::SaSaltUseSalesTax,
                    addr: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );

        // With the parents present, the same edits succeed and stick.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::Spouse,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::BlindSpouse,
                addr: RowAddr::default(),
                value: FieldValue::TriState(Some(true)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref().unwrap().header.spouse.as_ref().unwrap().blind,
            Some(true)
        );
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::SaSaltUseSalesTax,
                addr: RowAddr::default(),
                value: FieldValue::TriState(Some(false)),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(
            w.as_ref()
                .unwrap()
                .schedule_a
                .as_ref()
                .unwrap()
                .salt_use_sales_tax,
            Some(false)
        );
    }

    /// ★ I-4: `AddRow`/`RemoveRow` report an absent parent / out-of-range row rather than silently no-op'ing.
    #[test]
    fn addrow_removerow_report_absent_parent_and_out_of_range_i4() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);

        // AddRow box-12 with parent [0] but no W-2 at index 0 → NoSuchRow (was a silent Ok no-op).
        assert_eq!(
            apply(
                &mut w,
                Edit::AddRow {
                    section: SectionId::W2Box12,
                    parent: RowAddr(vec![0])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // AddRow charitable with no schedule_a → NoSuchRow.
        assert_eq!(
            apply(
                &mut w,
                Edit::AddRow {
                    section: SectionId::ScheduleACharitable,
                    parent: RowAddr::default()
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // RemoveRow of a W-2 that isn't there → NoSuchRow.
        assert_eq!(
            apply(
                &mut w,
                Edit::RemoveRow {
                    section: SectionId::W2s,
                    addr: RowAddr(vec![3])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );

        // With a W-2 present, AddRow box-12 succeeds; removing an out-of-range box-12 still refuses.
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2Box12,
                parent: RowAddr(vec![0]),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box12.len(), 1);
        assert_eq!(
            apply(
                &mut w,
                Edit::RemoveRow {
                    section: SectionId::W2Box12,
                    addr: RowAddr(vec![0, 5])
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
    }

    /// ★ M-2: `guard_arity` is EXACT — a too-LONG addr (extra indices the section never reads) is refused,
    /// not silently accepted with the tail ignored.
    #[test]
    fn overlong_rowaddr_is_rejected_m2() {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
            time::macros::date!(2026 - 09 - 01),
        )
        .unwrap();

        // Box1Wages needs depth 1; a depth-3 addr is over-long → NoSuchRow.
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::Box1Wages,
                    addr: RowAddr(vec![0, 7, 9]),
                    value: FieldValue::Money(dec!(1)),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        // A singleton field needs depth 0; any index is over-long → NoSuchRow.
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::TpFirstName,
                    addr: RowAddr(vec![0]),
                    value: FieldValue::Text("x".into()),
                },
                time::macros::date!(2026 - 09 - 01)
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
    }
}
