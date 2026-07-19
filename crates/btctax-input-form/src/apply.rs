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
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::Usd;

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
pub fn apply(w: &mut Working, e: Edit) -> Result<(), ApplyError> {
    match w {
        // ★ NI-2: nothing exists yet — only the filing-status *choice* brings a return into being.
        None => match e {
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr,
                value: value @ FieldValue::Choice(_),
            } => {
                let (field, _) =
                    locate_field(FieldId::FilingStatus).ok_or(ApplyError::NoSuchSection)?;
                // Set the status on an otherwise-pure default; only assign `*w` on success, so a bad choice
                // (e.g. an unknown status string) leaves `*w` as `None` — nothing laundered.
                let mut ri = ReturnInputs::default();
                (field.set)(&mut ri, &addr, value).map_err(ApplyError::SetError)?;
                *w = Some(ri);
                Ok(())
            }
            _ => Err(ApplyError::WrongFirstEdit),
        },
        Some(ri) => apply_to(ri, e),
    }
}

/// Dispatch an edit against a materialized return.
fn apply_to(ri: &mut ReturnInputs, e: Edit) -> Result<(), ApplyError> {
    match e {
        Edit::SetField { id, addr, value } => {
            let (field, depth) = locate_field(id).ok_or(ApplyError::NoSuchSection)?;
            guard_arity(&addr, depth)?;
            (field.set)(ri, &addr, value).map_err(ApplyError::SetError)
        }
        Edit::ClearField { id, addr } => {
            let (field, depth) = locate_field(id).ok_or(ApplyError::NoSuchSection)?;
            guard_arity(&addr, depth)?;
            // ★ The un-answer path (spec §5.7 M-6, review I-1). An `Enum` has no empty state (this includes
            // `filing_status`) → `Immutable`, WITHOUT touching anything. Else, a field carrying a dedicated
            // `clear` (the 13 registry-delegating tri-state/date leaves — whose registry setter writes only a
            // definite yes/no, so it cannot un-answer) routes through it, writing its `Option` leaf to `None`.
            // Every plain field clears via its own `set(empty_for_kind)`.
            if let FieldKind::Enum(_) = field.kind {
                return Err(ApplyError::SetError(SetError::Immutable));
            }
            if let Some(clear) = field.clear {
                return clear(ri, &addr).map_err(ApplyError::SetError);
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
            (field.set)(ri, &addr, empty).map_err(ApplyError::SetError)
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
        SectionId::Dependents | SectionId::W2s | SectionId::ScheduleACharitable => 1,
        SectionId::W2Box12 => 2,
        SectionId::ReturnOptions
        | SectionId::Taxpayer
        | SectionId::Spouse
        | SectionId::Address
        | SectionId::ScheduleA
        | SectionId::Payments
        | SectionId::Declarations
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
                }
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
                apply(&mut w, e.clone()),
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
                }
            ),
            Err(ApplyError::SetError(SetError::WrongKind)),
        );
        assert!(
            w.is_none(),
            "no return may materialize from an unparseable filing-status choice"
        );
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::ItemizeElection,
                addr: RowAddr::default(),
                value: FieldValue::Choice("ForceItemize".into()),
            },
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
        )
        .unwrap();
        assert!(w.as_ref().unwrap().w2s[0].box12.is_empty());
        apply(
            &mut w,
            Edit::RemoveRow {
                section: SectionId::W2s,
                addr: RowAddr(vec![0]),
            },
        )
        .unwrap();
        assert!(w.as_ref().unwrap().w2s.is_empty());

        // Spouse optional-singleton create → set → delete.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::Spouse,
            },
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
        )
        .unwrap();
        assert!(w.as_ref().unwrap().header.spouse.is_none());

        // Schedule A optional-singleton create → delete.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
        )
        .unwrap();
        assert!(w.as_ref().unwrap().schedule_a.is_some());
        apply(
            &mut w,
            Edit::DeleteSection {
                section: SectionId::ScheduleA,
            },
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
                }
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
        )
        .unwrap();
        assert_eq!(
            apply(
                &mut w,
                Edit::SetField {
                    id: FieldId::Box12Amount,
                    addr: RowAddr(vec![0]),
                    value: FieldValue::Money(dec!(1)),
                }
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
                }
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
                }
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
                }
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
    }

    /// ★ ClearField per-kind (review I-1/I-2, updated from the old "registry limitation" that pinned the bug):
    /// Enum → `Immutable`; the 13 registry-delegating tri-state/date fields UN-ANSWER their underlying `Option`
    /// leaf to `None` (spec §5.7 M-6); plain Date/Money/Text/Bool/Secret clear to their empty value; and the
    /// `IpPin` Secret clears to `None` (never `Some("")`).
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
                }
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
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().foreign_accounts, Some(true));
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::DeclForeignAccounts,
                addr: RowAddr::default(),
            },
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
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().header.ip_pin.as_deref(), Some("112233"));
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::IpPin,
                addr: RowAddr::default(),
            },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::DepDob,
                addr: RowAddr(vec![0]),
                value: FieldValue::Date(Some(date!(2015 - 06 - 01))),
            },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(500)),
            },
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
            },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpFirstName,
                addr: RowAddr::default(),
            },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpPresidentialFund,
                addr: RowAddr::default(),
            },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::TpSsn,
                addr: RowAddr::default(),
            },
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
                }
            ),
            Err(ApplyError::NoSuchSection),
        );
        // CreateSection on a non-optional section is a clean refusal.
        assert_eq!(
            apply(
                &mut w,
                Edit::CreateSection {
                    section: SectionId::Payments
                }
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
            )
            .unwrap();
            apply(
                &mut w,
                Edit::ClearField {
                    id,
                    addr: RowAddr::default(),
                },
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::ClearField {
                id: FieldId::BlindTaxpayer,
                addr: RowAddr::default(),
            },
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
                }
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
                }
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
                }
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
        assert_eq!(
            apply(
                &mut w,
                Edit::ClearField {
                    id: FieldId::SaSaltUseSalesTax,
                    addr: RowAddr::default()
                }
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );

        // With the parents present, the same edits succeed and stick.
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::Spouse,
            },
        )
        .unwrap();
        apply(
            &mut w,
            Edit::CreateSection {
                section: SectionId::ScheduleA,
            },
        )
        .unwrap();
        apply(
            &mut w,
            Edit::SetField {
                id: FieldId::BlindSpouse,
                addr: RowAddr::default(),
                value: FieldValue::TriState(Some(true)),
            },
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
                }
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
                }
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
                }
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
        )
        .unwrap();
        apply(
            &mut w,
            Edit::AddRow {
                section: SectionId::W2Box12,
                parent: RowAddr(vec![0]),
            },
        )
        .unwrap();
        assert_eq!(w.as_ref().unwrap().w2s[0].box12.len(), 1);
        assert_eq!(
            apply(
                &mut w,
                Edit::RemoveRow {
                    section: SectionId::W2Box12,
                    addr: RowAddr(vec![0, 5])
                }
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
                }
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
                }
            ),
            Err(ApplyError::SetError(SetError::NoSuchRow)),
        );
    }
}
