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
    current_prompt, dependent_ssn_hash, forget_answer, record_answer, AnswerKey, AnswerState,
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
/// Derived from the THREE registry maps that already exist ([`crate::spec::field_to_question`] /
/// [`crate::spec::field_to_skippable`] / [`crate::spec::field_to_dependent_gate`]), so the set of
/// fields that RECORD is the set of fields that delegate to a registry question — by identity, not
/// by a second hand-written list that can drift.
///
/// ★★★ **FR-97 — it takes the ROW and the RETURN, and that is why it was a seam change.** A
///     dependent gate's key is `AnswerKey::DependentGate { ssn_hash, gate }`: the gate names the
///     question and the row's own SSN names *whose* answer it is. Neither is knowable from a bare
///     `FieldId`, so before this the twenty `DepGate*` fields (and `DepDob`, the twenty-first gate)
///     returned `None` here — `Edit::SetField` set the leaf and recorded **nothing**, while `income
///     answer` recorded the identical answer in full. Two surfaces, two provenances, and the
///     difference invisible on the printed return.
///
/// ★ The `addr` is the DEPENDENTS row address, so it must be the one the edit was applied at, and
///   the `ri` must be read AFTER the set (a gate `set` never touches the `ssn`, so the identity is
///   the same either way; reading it after keeps one rule for `SetField` and `ClearField` both).
///   A row the addr does not name is `None` — no row, no identity, no record.
#[must_use]
pub fn answer_key_for(id: FieldId, ri: &ReturnInputs, addr: &RowAddr) -> Option<AnswerKey> {
    if let Some(q) = crate::spec::field_to_question(id) {
        return Some(AnswerKey::Question(q));
    }
    if let Some(s) = crate::spec::field_to_skippable(id) {
        return Some(AnswerKey::Skippable(s));
    }
    let gate = crate::spec::field_to_dependent_gate(id)?;
    let row = ri.header.dependents.get(*addr.0.first()?)?;
    Some(AnswerKey::DependentGate {
        ssn_hash: dependent_ssn_hash(&row.ssn),
        gate,
    })
}

/// ★★★ **The words THIS SURFACE put in front of the filer** — the comparand `record_answer` hashes.
///
/// For a question or a skippable that is [`current_prompt`], the one resolver R10.4/M-4 leaves.
/// A dependent gate resolves through the same registry, with `params: None`, **because the form seam
/// has no year package** — and the registry's words are what every other surface resolves too
/// (`current_prompt`, and `income answer`), so a record written here reads as `Given` rather than as
/// `WordingChanged` the instant it is written. Hashing anything else is seam review C-1's brick.
/// Each tri-state gate `Field` draws exactly this sentence (`dep_gate_tristate!` takes its `label`
/// from the registry); `DepDob` draws a shorter caption, which is FR-100 and a rendering item.
///
/// ★★ **The consequence, stated rather than hidden.** `DependentGate::GrossIncomeUnderLimit` quotes
///    the year's §152(d)(1)(B) figure, so with no package its words are the FIGURELESS fallback
///    (*"…WAITING ON THE TAX YEAR'S PARAMETER PACKAGE…"*, FR-83). An answer given to that sentence in
///    the editor therefore does NOT stand once the figure is known: `interview_state_with_params` and
///    `screen_dependent_gates` compare the stored hash against the RENDERED question and re-ask it.
///    That is R10.3 working — *"an answer given under earlier words does not stand under later
///    ones"* — and it is strictly better than the alternative this replaces, which was to record
///    nothing and let the gate count as answered under words nobody was shown. `income answer` holds
///    the package and asks the real question; the editor's own `ClearField` un-answers it.
///    ★ `current_prompt` returns `None` for that one gate on purpose, so it cannot be used here.
fn prompt_for_record(key: &AnswerKey, ri: &ReturnInputs) -> Option<std::borrow::Cow<'static, str>> {
    match key {
        AnswerKey::DependentGate { gate, .. } => {
            Some(btctax_core::tax::dependent_gates::entry(*gate).prompt_text(ri, None))
        }
        _ => current_prompt(key, ri),
    }
}

/// ★★★ **Record what the edit actually left on the row — provenance FOLLOWS the leaf.**
///
/// The twenty tri-state gates cannot be set to "unanswered" (`Field::set` refuses a
/// `TriState(None)`), but the twenty-first — `DepDob`, a plain `Date` leaf — accepts `Date(None)`,
/// and a `Given` record for a date that is not there would be a record of testimony nobody gave
/// (*"an entry is testimony"*). So a set that leaves a gate ANSWERED records, and one that empties
/// it forgets, exactly as `ClearField` does. Non-gate keys are unaffected: their `set` cannot land
/// on a row at all.
fn record_or_forget(ri: &mut ReturnInputs, key: AnswerKey, addr: &RowAddr, now: Date) {
    if let AnswerKey::DependentGate { gate, .. } = &key {
        let answered = addr
            .0
            .first()
            .and_then(|i| ri.header.dependents.get(*i))
            .is_some_and(|d| btctax_core::tax::dependent_gates::gate_is_answered(d, *gate));
        if !answered {
            forget_answer(ri, &key);
            return;
        }
    }
    // ★ R10.4 — the words are RENDERED FROM THE RETURN, so a question that quotes a value hashes
    //   the sentence the filer actually saw. Owned first, because `record_answer` takes `&mut ri`
    //   and the rendered text borrows it.
    let prompt = prompt_for_record(&key, ri).map(std::borrow::Cow::into_owned);
    if let Some(prompt) = prompt {
        record_answer(ri, key, &prompt, now, AnswerState::Given);
    }
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
            if let Some(key) = answer_key_for(id, ri, &addr) {
                record_or_forget(ri, key, &addr, now);
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
                if let Some(key) = answer_key_for(id, ri, &addr) {
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
            if let Some(key) = answer_key_for(id, ri, &addr) {
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
        | SectionId::BrokerReporting
        // ★ R4 / T5 — the six document sections are depth-1 repeating groups over a TOP-LEVEL
        //   `Vec`, exactly like `W2s`.
        | SectionId::Int1099s
        | SectionId::Div1099s
        | SectionId::B1099s
        | SectionId::G1099s
        | SectionId::Form1098Es
        // ★ R4 / R8 / T9 — the Form 1098 rows, same shape.
        | SectionId::Form1098s
        // ★ R8 / T9 — Schedule A line 8b's recipient rows. Depth 1 too, even though the `Vec` hangs
        //   off the OPTIONAL `schedule_a`: the depth is how many indices name a row, and there is
        //   one. A missing Schedule A is `NoSuchRow` at the accessor, not another level.
        | SectionId::NonForm1098Interest
        // ★ R4 / T16 — the two HSA information returns, same shape.
        | SectionId::Sa1099s
        | SectionId::Sa5498s
        | SectionId::ScheduleBFilerRecords => 1,
        SectionId::W2Box12 => 2,
        SectionId::ReturnOptions
        | SectionId::Taxpayer
        | SectionId::Spouse
        | SectionId::Address
        // ★ T10 / §5.4 — the direct-deposit block is an OPTIONAL SINGLETON, like `Spouse`: one
        //   instruction per return, present or absent, never a row.
        | SectionId::DirectDeposit
        | SectionId::ScheduleA
        | SectionId::Payments
        | SectionId::Carryforwards
        | SectionId::QbiLimitation
        | SectionId::Declarations
        // ★ R3 — the census is a SINGLETON: one tri-state per document TYPE, not per document. The
        //   per-document rows live in the document sections (`W2s`, and T5's 1099 sections).
        | SectionId::DocumentCensus
        // ★ T16 — Form 8889's money leaves are a SINGLETON: one HSA surface per return. (Two
        //   spouses with separate HSAs need two Forms 8889, which REFUSES.)
        | SectionId::Form8889
        // ★ R8 / T9 — the sale of a main home is a SINGLETON: one main home, four answers.
        | SectionId::HomeSale
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
        // ★ T9 — prime a Form 1098 ROW so `SaMortgageAllUsed` is live (its set/clear gate on
        //   `mortgage_question_live`, now `schedule_a.is_some() && !form_1098.is_empty()`).
        w.as_mut().expect("materialized").form_1098 =
            vec![btctax_core::tax::testonly::form_1098_with_interest(dec!(
                1000
            ))];
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

    // ── ★★★ FR-97 — THE ANSWER-LOG SEAM: THE TWENTY-ONE DEPENDENT GATES RECORD HERE TOO ──────────
    //
    // `answer_key_for` returned `None` for every `DepGate*` field (and for `DepDob`), so an answer
    // given in the EDITOR set the leaf and recorded nothing while `income answer` recorded the
    // identical answer in full. The five tests below hold the fix at the layer the filer enters:
    // through `apply(Edit::SetField)`, never by assembling an `AnswerRecord` by hand.

    /// A working return carrying ONE dependent row at `RowAddr(vec![0])`, built THROUGH the seam
    /// (materialize → `AddRow` → `SetField`) so nothing here reaches past the layer under test.
    ///
    /// ★ The SSN is from the never-issued space (area 000), and the year is set directly because the
    ///   tax year is not a form `Field` at all — the renderer carries it (`TaxInputsFormState::fresh`).
    fn with_one_dependent(ssn: &str) -> Working {
        let mut w: Working = None;
        materialize(&mut w, FilingStatus::Single);
        w.as_mut().expect("materialized").tax_year = 2024;
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::Dependents,
                parent: RowAddr::default(),
            },
            NOW,
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::DepSsn,
                addr: RowAddr(vec![0]),
                value: FieldValue::SecretEntry(ssn.to_string()),
            },
            NOW,
        )
        .unwrap();
        w
    }

    /// The date this module's FR-97 tests answer at. One constant, so a record's `answered_on` is
    /// checkable against the seam's own date rather than a wall clock.
    const NOW: time::Date = time::macros::date!(2026 - 09 - 01);

    /// ★★★ **FR-97's COVERAGE CHECK, AND IT IS DERIVED — no hand-written list of what records.**
    ///
    /// FR-99's shape (seven instances in this arc) is *a hand list standing beside a set that grows*:
    /// every one was right when written and wrong after a later task widened the set. So this does not
    /// enumerate the fields that record. It asserts an EQUALITY between two derived sets:
    ///
    ///   * **expected** — the image of the three registry→form maps, over the three registries'
    ///     own totality (`FORM_QUESTIONS`, `SKIPPABLE_QUESTIONS`, `DependentGate::ALL`);
    ///   * **actual** — every `Field` in `form_spec()` for which `answer_key_for` yields a key.
    ///
    /// A twenty-second dependent gate is a compile error in `gate_to_field` until it is placed, then
    /// lands in `expected` here for free — and if `answer_key_for` ever stopped deriving from that map
    /// (the FR-97 defect, which was exactly this: twenty fields in a registry and none of them in the
    /// key function), the two sets separate and this reds with both differences printed.
    ///
    /// ★ **What it does NOT check**: whether a field that records records the RIGHT key, or the right
    ///   words. That is the next three tests' job, and the cross-surface byte-comparison in
    ///   `btctax-cli`'s `tax_report.rs`.
    #[test]
    fn the_fields_that_record_an_answer_are_exactly_the_three_registries_images() {
        use btctax_core::tax::provenance::{dependent_ssn_hash, DependentGate};
        use btctax_core::tax::questions::{FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
        use std::collections::BTreeSet;

        const SSN: &str = "000-00-0000";
        let w = with_one_dependent(SSN);
        let ri = w.as_ref().expect("materialized");
        let addr = RowAddr(vec![0]);

        let name = |id: FieldId| format!("{id:?}");
        let expected: BTreeSet<String> = FORM_QUESTIONS
            .iter()
            .map(|q| crate::spec::question_to_field(q.id))
            .chain(
                SKIPPABLE_QUESTIONS
                    .iter()
                    .map(|s| crate::spec::skippable_to_field(s.id)),
            )
            .chain(
                DependentGate::ALL
                    .iter()
                    .map(|g| crate::spec::gate_to_field(*g)),
            )
            .map(name)
            .collect();
        let actual: BTreeSet<String> = crate::spec::form_spec()
            .iter()
            .flat_map(|s| s.fields.iter())
            .filter(|f| super::answer_key_for(f.id, ri, &addr).is_some())
            .map(|f| name(f.id))
            .collect();
        assert_eq!(
            expected.difference(&actual).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "a registry owns these fields and `answer_key_for` records NONE of them — an answer \
             given in the editor would set the leaf and write no provenance at all (FR-97)"
        );
        assert_eq!(
            actual.difference(&expected).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "these fields record an answer that no registry owns — a key nothing can ever resolve \
             back to a question"
        );

        // ★ …and the GATES resolve to the row's own identity, which is the half a set comparison
        //   cannot see: the key must name WHOSE answer it is.
        for g in DependentGate::ALL.iter().copied() {
            assert_eq!(
                super::answer_key_for(crate::spec::gate_to_field(g), ri, &addr),
                Some(btctax_core::tax::provenance::AnswerKey::DependentGate {
                    ssn_hash: dependent_ssn_hash(SSN),
                    gate: g,
                }),
                "{g:?} must key on the ROW's salted SSN, exactly as `income answer` keys it"
            );
        }
    }

    /// Answer, through the seam, every gate the walk currently DEMANDS — repeating until the walk
    /// stops growing, because answering one gate is what makes the next live. Each yes/no takes the
    /// gate's own declared `claim_path` polarity, except the gates named in `flip`, which take its
    /// negation (that is how one row is steered onto the qualifying-RELATIVE branch).
    ///
    /// ★ Nothing here reaches past `apply`: the fixture is built by the same `Edit`s a keystroke
    ///   produces, and the gate set comes from `walk_dependent`'s own demands rather than a list.
    fn answer_demanded_gates(
        w: &mut Working,
        dob: time::Date,
        flip: &[btctax_core::tax::provenance::DependentGate],
    ) {
        use btctax_core::tax::dependent_gates::{
            entry, gate_is_answered, walk_dependent, GateKind, DEPENDENT_GATES,
        };
        for _ in 0..=DEPENDENT_GATES.len() {
            let demanded = walk_dependent(w.as_ref().expect("materialized"), 0).demanded_gates();
            let mut progressed = false;
            for g in demanded {
                if gate_is_answered(&w.as_ref().unwrap().header.dependents[0], g) {
                    continue;
                }
                let q = entry(g);
                let value = match q.kind {
                    GateKind::Date => FieldValue::Date(Some(dob)),
                    GateKind::YesNo => {
                        let yes = q.claim_path.expect("a YesNo gate declares its claim path");
                        FieldValue::TriState(Some(if flip.contains(&g) { !yes } else { yes }))
                    }
                };
                apply(
                    w,
                    Edit::SetField {
                        id: crate::spec::gate_to_field(g),
                        addr: RowAddr(vec![0]),
                        value,
                    },
                    NOW,
                )
                .unwrap_or_else(|e| {
                    panic!("{g:?} is demanded, so the editor must accept it: {e:?}")
                });
                progressed = true;
            }
            if !progressed {
                return;
            }
        }
        panic!("the gate walk never settled");
    }

    /// ★★★ **FR-97's KILL — every gate the flowchart DEMANDS, answered through `apply`, is recorded
    ///     under the row's identity and hashes the REGISTRY's words.**
    ///
    /// It enters where the filer enters (`Edit::SetField`) and derives both halves it checks: the
    /// gate set from `walk_dependent`'s own demands, and the expected hash from the gate registry —
    /// which is also what `income answer` hashes and what `current_prompt` resolves, so a record
    /// written here reads as `Given` rather than as `WordingChanged` the instant it is written
    /// (seam review C-1's brick, from the other side).
    #[test]
    fn every_demanded_dependent_gate_answered_through_apply_records_the_registrys_words() {
        use btctax_core::tax::dependent_gates::{entry, walk_dependent};
        use btctax_core::tax::provenance::{
            answer_status, dependent_ssn_hash, prompt_hash, AnswerKey, AnswerState, AnswerStatus,
        };

        const SSN: &str = "000-00-0001";
        let mut w = with_one_dependent(SSN);
        answer_demanded_gates(&mut w, date!(2014 - 06 - 01), &[]);

        let ri = w.as_ref().expect("materialized");
        let demanded = walk_dependent(ri, 0).demanded_gates();
        assert!(
            demanded.len() >= 10,
            "the premise: the qualifying-child path demands a real gate set, not one or two: \
             {demanded:?}"
        );
        for g in demanded {
            let key = AnswerKey::DependentGate {
                ssn_hash: dependent_ssn_hash(SSN),
                gate: g,
            };
            let rec = ri.answer_log.get(&key).unwrap_or_else(|| {
                panic!(
                    "{g:?} was answered in the EDITOR and recorded nothing — the row counts as \
                     answered under words it may never have been shown (FR-97)"
                )
            });
            assert_eq!(
                rec.answered_on, NOW,
                "{g:?}: the seam's date, not a wall clock"
            );
            assert_eq!(rec.state, AnswerState::Given, "{g:?}");
            assert_eq!(
                rec.prompt_hash,
                prompt_hash(&entry(g).prompt_text(ri, None)),
                "{g:?}: the record must hash the gate REGISTRY's words — the same comparand \
                 `current_prompt` resolves and `income answer` writes"
            );
            assert_eq!(
                answer_status(ri, &key),
                AnswerStatus::Given,
                "{g:?}: …and it must therefore read as ANSWERED, not as re-ask-me"
            );
        }
    }

    /// ★★★ **WHAT THE FILER SAW vs WHAT WAS HASHED — the one gate where they differ, named with its
    ///     reason rather than left to be discovered.**
    ///
    /// Hashing something other than the words on the screen is seam review C-1, so the correspondence
    /// is checked rather than assumed: every `GateKind::YesNo` gate's `Field` draws the registry
    /// prompt verbatim (the `dep_gate_tristate!` macro takes `label` from the registry), so the
    /// hashed sentence IS the drawn one. `GateKind::Date` — `DateOfBirth` alone, and the registry
    /// says so — is the exception: `DepDob` is a plain leaf that predates T7's gate registry and
    /// draws the caption *"Date of birth"*, while the record hashes the registry's question.
    ///
    /// That is deliberate and it is the only correct choice: `current_prompt` and `income answer`
    /// both resolve the registry's words, so hashing the caption instead would make the editor's own
    /// answer read `WordingChanged` forever. The residue — a caption and a question that are not the
    /// same sentence — is a RENDERER wording item, recorded in `FOLLOWUPS.md` (FR-100), not a
    /// provenance defect.
    #[test]
    fn the_gate_fields_draw_the_words_they_hash_except_the_one_named_date_leaf() {
        use btctax_core::tax::dependent_gates::{entry, GateKind};
        use btctax_core::tax::provenance::DependentGate;

        let w = with_one_dependent("000-00-0004");
        let ri = w.as_ref().expect("materialized");
        let label_of = |id: FieldId| {
            crate::spec::form_spec()
                .iter()
                .flat_map(|s| s.fields.iter())
                .find(|f| f.id == id)
                .expect("every gate has a Field")
                .label
        };
        let mut different = Vec::new();
        for g in DependentGate::ALL.iter().copied() {
            let drawn = label_of(crate::spec::gate_to_field(g));
            let hashed = entry(g).prompt_text(ri, None);
            if drawn != hashed {
                different.push((g, entry(g).kind));
            }
        }
        assert_eq!(
            different
                .iter()
                .filter(|(_, k)| *k == GateKind::YesNo)
                .collect::<Vec<_>>(),
            Vec::<&(DependentGate, GateKind)>::new(),
            "a yes/no gate that draws one sentence and hashes another is C-1 again: the filer's \
             answer would read as given under words they never saw"
        );
        assert_eq!(
            different.iter().map(|(g, _)| *g).collect::<Vec<_>>(),
            vec![DependentGate::DateOfBirth],
            "exactly ONE gate may draw words other than the ones it hashes, and it is the plain \
             `Date` leaf that predates the registry — a second one is a new defect, not a new \
             exception"
        );
    }

    /// ★★★ **Provenance follows the LEAF: emptying the date of birth un-answers it.**
    ///
    /// `DepDob` is the one gate whose `set` accepts an empty value (`Date(None)`), and a `Given`
    /// record dated today for a date that is not on the row would be a record of testimony nobody
    /// gave — the same act `Durability::Durable` refuses at the keyboard, where a bare Enter on a
    /// seeded row leaves *"no value, no record, and the gate still blocking"*.
    #[test]
    fn emptying_a_dependents_date_of_birth_forgets_its_record_rather_than_dating_a_blank() {
        use btctax_core::tax::provenance::{dependent_ssn_hash, AnswerKey, DependentGate};

        const SSN: &str = "000-00-0002";
        let mut w = with_one_dependent(SSN);
        let key = AnswerKey::DependentGate {
            ssn_hash: dependent_ssn_hash(SSN),
            gate: DependentGate::DateOfBirth,
        };
        let set = |w: &mut Working, v: Option<time::Date>| {
            apply(
                w,
                Edit::SetField {
                    id: FieldId::DepDob,
                    addr: RowAddr(vec![0]),
                    value: FieldValue::Date(v),
                },
                NOW,
            )
            .unwrap();
        };
        set(&mut w, Some(date!(2014 - 06 - 01)));
        assert!(
            w.as_ref().unwrap().answer_log.contains_key(&key),
            "a date TYPED in the editor is testimony, and it is recorded"
        );
        set(&mut w, None);
        assert!(
            !w.as_ref().unwrap().answer_log.contains_key(&key),
            "…and emptying it takes the record with it: a record for a blank date is testimony \
             nobody gave"
        );
        assert!(
            w.as_ref().unwrap().answer_log_history.is_empty(),
            "un-answering writes no history (§5.6 holds superseded WORDINGS, not withdrawals)"
        );
    }

    /// ★★★ **THE BOUNDARY, STATED AS A TEST — the one gate whose words the editor cannot state.**
    ///
    /// `GrossIncomeUnderLimit` quotes the year's §152(d)(1)(B) figure, and the form seam holds no
    /// `FullReturnParams` (`Field.live` has none, and a gate `Field`'s label IS the registry's static
    /// prompt). So the editor draws FR-83's figureless fallback — *"…WAITING ON THE TAX YEAR'S
    /// PARAMETER PACKAGE…"* — and records THAT, which is the only honest comparand: it is what the
    /// filer read.
    ///
    /// The consequence is R10.3 doing its job rather than a defect hidden: once the package is in
    /// hand the stored hash disagrees with the rendered question, so the panel lists the gate as
    /// blocking and `screen_dependent_gates` refuses. `income answer` holds the package and asks the
    /// real question; `ClearField` withdraws the editor's answer. What this must never be again is
    /// the pre-FR-97 behaviour — **no record at all**, which let the gate count as ANSWERED under
    /// words nobody was shown.
    #[test]
    fn the_params_quoting_gate_records_the_fallback_it_drew_and_is_re_asked_once_the_figure_lands()
    {
        use btctax_core::tax::dependent_gates::{entry, walk_dependent};
        use btctax_core::tax::provenance::{
            dependent_ssn_hash, prompt_hash, AnswerKey, DependentGate,
        };

        const SSN: &str = "000-00-0003";
        let mut w = with_one_dependent(SSN);
        // The qualifying-RELATIVE path — Step 1's relationship test answered against its claim path
        // sends the row to Step 4, which is where the gross income test lives. Every answer goes
        // through the seam, and the gate SET comes from the walk's own demands.
        answer_demanded_gates(
            &mut w,
            date!(1950 - 03 - 04),
            &[DependentGate::QcRelationship],
        );
        let ri = w.as_ref().unwrap();
        assert!(
            walk_dependent(ri, 0).demands(DependentGate::GrossIncomeUnderLimit),
            "the premise: this row is on the qualifying-relative path, where Step 4 asks the gross \
             income test"
        );

        let key = AnswerKey::DependentGate {
            ssn_hash: dependent_ssn_hash(SSN),
            gate: DependentGate::GrossIncomeUnderLimit,
        };
        let rec = ri
            .answer_log
            .get(&key)
            .expect("an answer given in the editor is testimony, and testimony gets provenance");
        assert_eq!(
            rec.prompt_hash,
            prompt_hash(&entry(DependentGate::GrossIncomeUnderLimit).prompt_text(ri, None)),
            "the record hashes the FIGURELESS fallback, because that is the sentence the pane drew"
        );

        // …and with the year's package in hand, that answer does NOT stand: the question the filer
        // must answer quotes a figure their screen never showed.
        let params = btctax_core::tax::testonly::ty2024_params();
        let st = btctax_core::tax::interview_state::interview_state_with_params(ri, &params);
        assert!(
            st.blocking.iter().any(|b| b.item == key
                && b.reason == btctax_core::tax::provenance::WORDING_CHANGED_REASON),
            "R10.3 — an answer given under earlier words does not stand under later ones: {:?}",
            st.blocking
        );
        // The editor's own withdrawal works: clearing returns it to never-asked.
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::DepGateGrossIncomeUnderLimit,
                addr: RowAddr(vec![0]),
            },
            NOW,
        )
        .unwrap();
        assert!(
            !w.as_ref().unwrap().answer_log.contains_key(&key),
            "`ClearField` un-answers a gate — the record goes with the leaf"
        );
    }
}
