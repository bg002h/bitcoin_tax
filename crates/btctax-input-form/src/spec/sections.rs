//! ★ The ten non-synthetic `FormSpec` sections (spec §5.8) — one `Field` per header/W-2/Schedule-A/... leaf,
//! over the `ReturnInputs` tax struct. Every accessor is a NON-CAPTURING closure (it reads only its `ri`/`addr`
//! params and `const` paths), so it coerces to the bare `fn` pointer `Field` requires — no `serde_json::Value`
//! reflection, no per-row generics (spec §4). Assembled with Task 4's two synthetic sections in [`super`]'s
//! `form_spec()`.
//!
//! The two registry-driven Schedule-A tri-states (`SaSaltUseSalesTax`, `SaMortgageAllUsed`) do NOT get a
//! hand-rolled accessor: they DELEGATE, via Task 4's `skippable_tristate!`/`decl_tristate!` macros, to the
//! same `SKIPPABLE_QUESTIONS`/`FORM_QUESTIONS` entries the `FieldId ↔ registry` maps already tie them to — one
//! liveness predicate, one accessor, per concept (spec §13). Task 6's coverage KAT and Task 9's attribution
//! depend on that single source.

use crate::seam::{
    Field, FieldId, FieldKind, FieldValue, SecretView, Section, SectionId, SectionKind, SetError,
};
use btctax_core::conventions::Usd;
use btctax_core::forms::{BrokerReported, Cohort};
use btctax_core::tax::dependent_gates::DEPENDENT_GATES;
use btctax_core::tax::questions::{FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
use btctax_core::tax::return_inputs::{
    Box12Entry, CharitableClass, CharitableGift, Dependent, ItemizeElection, Person,
    ScheduleAInputs, W2,
};
use btctax_core::tax::types::FilingStatus;

// ── The Secret maskers — the ONLY constructors of `SecretView::Set` in this crate (via `set_masked`) ────────
// A caller cannot store raw digits through them: each emits only a fixed presence shape (spec §4/§5.5), and
// both route through the guarded `SecretView::set_masked`. The discipline is PER SECRET (review I-3):
//   • SSN → `***-**-NNNN`, revealing the last four ONLY for a canonical 9-digit input (matching the CLI's
//     `mask_ssn` and `mask_pii`); any other length is fully masked to `***-**-****` (no digits of a
//     non-canonical secret leak — folds follow-up (k)'s short-input reveal).
//   • IP PIN (and any non-SSN secret) → a fixed all-`*` token with ZERO digits, mirroring the `IpPin(******)`
//     Debug discipline (a 6-digit anti-fraud PIN has a search space of 10^6 — last-4 would leak 2/3 of it).
// Empty input → `Empty`. Canonical validation of the raw entry is `parse`, upstream of the `SecretEntry`.
fn mask_ssn(raw: &str) -> SecretView {
    if raw.is_empty() {
        return SecretView::Empty;
    }
    if raw.chars().count() == 9 && raw.chars().all(|c| c.is_ascii_digit()) {
        let last4: String = raw.chars().skip(5).collect();
        SecretView::set_masked(format!("***-**-{last4}"))
    } else {
        SecretView::set_masked("***-**-****".to_string())
    }
}

fn mask_ip_pin(raw: &str) -> SecretView {
    if raw.is_empty() {
        return SecretView::Empty;
    }
    SecretView::set_masked("******".to_string())
}

// ── Leaf generators for the repetitive money families (one struct-field ident is all that varies) ──────────

/// A repeating-W-2-row `Money` leaf over `ri.w2s[addr.0[0]].$field`.
macro_rules! w2_money {
    ($id:expr, $label:literal, $help:literal, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Money(w.$field)),
            set: |ri, a, v| {
                let FieldValue::Money(m) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.$field = m;
                Ok(())
            },
        }
    };
}

/// An optional-Schedule-A `Money` leaf over `ri.schedule_a.$field` (get `None` / set `NoSuchRow` when absent).
macro_rules! scha_money {
    ($id:expr, $label:literal, $help:literal, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, _| ri.schedule_a.as_ref().map(|a| FieldValue::Money(a.$field)),
            set: |ri, _, v| {
                let FieldValue::Money(m) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.schedule_a.as_mut().ok_or(SetError::NoSuchRow)?.$field = m;
                Ok(())
            },
        }
    };
}

/// A `Money` leaf directly on `ReturnInputs` (no optional parent, no row).
macro_rules! ret_money {
    ($id:expr, $label:literal, $help:literal, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, _| Some(FieldValue::Money(ri.$field)),
            set: |ri, _, v| {
                let FieldValue::Money(m) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.$field = m;
                Ok(())
            },
        }
    };
}

// ── §911/931/933 exclusions — the MAGI add-backs ─────────────────────────────────────────────────
//
// ★ ONE quantity, FIVE phase-outs. §164(b)(7)(B)(iv) defines modified AGI as "adjusted gross income
// increased by any amount excluded from gross income under section 911, 931, or 933", and the same
// four amounts appear at the SALT worksheet's lines 3a-3d and Schedule 1-A Part I's lines 2a-2d. They
// live on `ReturnInputs` rather than `ScheduleAInputs` precisely because a standard-deduction filer
// with qualified tips needs them too.
//
// The yes/no that carries answered-ness is `QuestionId::HasIncomeExclusion`, in the Declarations
// section; these are the amounts it gates.
pub(crate) const INCOME_EXCLUSION_FIELDS: &[Field] = &[
    ret_money!(
        FieldId::ExclPuertoRico,
        "Excluded Puerto Rico income",
        "Schedule 1-A line 2a / SALT worksheet line 3a - income from Puerto Rico you excluded (\u{a7}933).",
        excluded_puerto_rico_income
    ),
    ret_money!(
        FieldId::Excl2555L45,
        "Form 2555 line 45",
        "Schedule 1-A line 2b / SALT worksheet line 3b - foreign earned income exclusion (\u{a7}911).",
        form_2555_line45
    ),
    ret_money!(
        FieldId::Excl2555L50,
        "Form 2555 line 50",
        "Schedule 1-A line 2c / SALT worksheet line 3c - foreign housing exclusion (\u{a7}911).",
        form_2555_line50
    ),
    ret_money!(
        FieldId::Excl4563L15,
        "Form 4563 line 15",
        "Schedule 1-A line 2d / SALT worksheet line 3d - American Samoa exclusion (\u{a7}931).",
        form_4563_line15
    ),
];

// ── 1. ReturnOptions (Singleton) — the filing-status/itemize-election header ────────────────────────────────

const RETURN_OPTIONS_FIELDS: &[Field] = &[
    Field {
        id: FieldId::FilingStatus,
        clear: None,
        label: "Filing status",
        help: "Single / MFJ / MFS / HoH / QSS (§1). Choosing it materializes the working return.",
        kind: FieldKind::Enum(&["Single", "Mfj", "Mfs", "HoH", "Qss"]),
        live: |_| true,
        get: |ri, _| Some(FieldValue::Choice(format!("{:?}", ri.filing_status))),
        set: |ri, _, v| {
            let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
            ri.filing_status = match c.as_str() {
                "Single" => FilingStatus::Single,
                "Mfj" => FilingStatus::Mfj,
                "Mfs" => FilingStatus::Mfs,
                "HoH" => FilingStatus::HoH,
                "Qss" => FilingStatus::Qss,
                _ => return Err(SetError::WrongKind),
            };
            Ok(())
        },
    },
    Field {
        id: FieldId::ItemizeElection,
        clear: None,
        label: "Itemize election",
        help: "Auto = take the larger of standard vs Schedule A; ForceItemize = §63(e) elect to itemize \
               even if smaller.",
        kind: FieldKind::Enum(&["Auto", "ForceItemize"]),
        // ★ I-5 (spec §5.8): live ONLY with a Schedule A. Otherwise a renderer would show the election on a
        // no-Schedule-A return, a filer forces itemize, and commit produces a $0-deduction return (the
        // standard deduction silently forfeited — the §5.1/I-10 hazard through the set door).
        live: |ri| ri.schedule_a.is_some(),
        get: |ri, _| Some(FieldValue::Choice(format!("{:?}", ri.itemize_election))),
        set: |ri, _, v| {
            let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
            ri.itemize_election = match c.as_str() {
                "Auto" => ItemizeElection::Auto,
                "ForceItemize" => ItemizeElection::ForceItemize,
                _ => return Err(SetError::WrongKind),
            };
            Ok(())
        },
    },
];

pub(crate) const RETURN_OPTIONS: Section = Section {
    id: SectionId::ReturnOptions,
    title: "Return options",
    kind: SectionKind::Singleton,
    fields: RETURN_OPTIONS_FIELDS,
};

// ── 2. Taxpayer (Singleton) — the primary filer's Person + the header IP PIN ────────────────────────────────

const TAXPAYER_FIELDS: &[Field] = &[
    Field {
        id: FieldId::TpFirstName,
        clear: None,
        label: "First name",
        help: "The taxpayer's legal first name (1040 header).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.taxpayer.first_name.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.taxpayer.first_name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::TpLastName,
        clear: None,
        label: "Last name",
        help: "The taxpayer's legal last name (1040 header).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.taxpayer.last_name.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.taxpayer.last_name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::TpSsn,
        clear: None,
        label: "SSN",
        help: "The taxpayer's Social Security number. Stored as entered; shown masked (`***-**-NNNN`).",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Secret(mask_ssn(&ri.header.taxpayer.ssn))),
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.taxpayer.ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::TpOccupation,
        clear: None,
        label: "Occupation",
        help: "The taxpayer's occupation (1040 signature block).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.taxpayer.occupation.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.taxpayer.occupation = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::TpPresidentialFund,
        clear: None,
        label: "Presidential Election Campaign Fund",
        help: "1040 header: check to direct $3 to the fund (does not change tax owed).",
        kind: FieldKind::Bool,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Bool(ri.header.presidential_fund_taxpayer)),
        set: |ri, _, v| {
            let FieldValue::Bool(b) = v else { return Err(SetError::WrongKind) };
            ri.header.presidential_fund_taxpayer = b;
            Ok(())
        },
    },
    Field {
        id: FieldId::IpPin,
        clear: None,
        label: "Identity Protection PIN",
        help: "The IRS-issued 6-digit IP PIN, if you have one. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| {
            Some(FieldValue::Secret(ri.header.ip_pin.as_deref().map_or(SecretView::Empty, mask_ip_pin)))
        },
        // ★ I-2: an empty entry maps to `None`, NEVER `Some("")` — a `Some("")` is screen-clean but bricks
        // `export` (`IpPin::canonical("")` errors) and renders identically to a healthy `None`. With I-1's
        // clear routing, both `ClearField(IpPin)` and `SetField(IpPin, SecretEntry(""))` now yield `None`.
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.ip_pin = if s.is_empty() { None } else { Some(s) };
            Ok(())
        },
    },
];

pub(crate) const TAXPAYER: Section = Section {
    id: SectionId::Taxpayer,
    title: "Taxpayer",
    kind: SectionKind::Singleton,
    fields: TAXPAYER_FIELDS,
};

// ── 3. Spouse (OptionalSingleton) — `header.spouse: Option<Person>` (+ the header presidential-fund bool) ───
// Every leaf is spouse-gated: get is `None` and set is `NoSuchRow` until the optional-singleton `create` runs.

const SPOUSE_FIELDS: &[Field] = &[
    Field {
        id: FieldId::SpFirstName,
        clear: None,
        label: "Spouse first name",
        help: "The spouse's legal first name (MFJ/MFS 1040 header).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| ri.header.spouse.as_ref().map(|s| FieldValue::Text(s.first_name.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.spouse.as_mut().ok_or(SetError::NoSuchRow)?.first_name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::SpLastName,
        clear: None,
        label: "Spouse last name",
        help: "The spouse's legal last name.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| ri.header.spouse.as_ref().map(|s| FieldValue::Text(s.last_name.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.spouse.as_mut().ok_or(SetError::NoSuchRow)?.last_name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::SpSsn,
        clear: None,
        label: "Spouse SSN",
        help: "The spouse's Social Security number. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| ri.header.spouse.as_ref().map(|s| FieldValue::Secret(mask_ssn(&s.ssn))),
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.spouse.as_mut().ok_or(SetError::NoSuchRow)?.ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::SpOccupation,
        clear: None,
        label: "Spouse occupation",
        help: "The spouse's occupation (1040 signature block).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| ri.header.spouse.as_ref().map(|s| FieldValue::Text(s.occupation.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.spouse.as_mut().ok_or(SetError::NoSuchRow)?.occupation = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::SpPresidentialFund,
        clear: None,
        label: "Spouse Presidential Election Campaign Fund",
        help: "1040 header: check to direct $3 to the fund for the spouse. Requires a spouse on the return.",
        kind: FieldKind::Bool,
        live: |_| true,
        // The bool lives on the header (not the Person), but the leaf is spouse-gated for section coherence.
        get: |ri, _| {
            ri.header.spouse.as_ref().map(|_| FieldValue::Bool(ri.header.presidential_fund_spouse))
        },
        set: |ri, _, v| {
            let FieldValue::Bool(b) = v else { return Err(SetError::WrongKind) };
            if ri.header.spouse.is_none() {
                return Err(SetError::NoSuchRow);
            }
            ri.header.presidential_fund_spouse = b;
            Ok(())
        },
    },
];

pub(crate) const SPOUSE: Section = Section {
    id: SectionId::Spouse,
    title: "Spouse",
    kind: SectionKind::OptionalSingleton {
        present: |ri| ri.header.spouse.is_some(),
        create: |ri| {
            if ri.header.spouse.is_none() {
                ri.header.spouse = Some(Person::default());
            }
        },
        delete: |ri| ri.header.spouse = None,
    },
    fields: SPOUSE_FIELDS,
};

// ── 4. Address (Singleton) — the four `header` address strings ──────────────────────────────────────────────

const ADDRESS_FIELDS: &[Field] = &[
    Field {
        id: FieldId::AddrStreet,
        clear: None,
        label: "Street address",
        help: "Home address — street (1040 header).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_street.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header.address_street = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrCity,
        clear: None,
        label: "City",
        help: "Home address — city or town.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_city.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header.address_city = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrState,
        clear: None,
        label: "State",
        help: "Home address — state (two-letter USPS code).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_state.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header.address_state = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrZip,
        clear: None,
        label: "ZIP code",
        help: "Home address — ZIP code.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_zip.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header.address_zip = s;
            Ok(())
        },
    },
];

pub(crate) const ADDRESS: Section = Section {
    id: SectionId::Address,
    title: "Address",
    kind: SectionKind::Singleton,
    fields: ADDRESS_FIELDS,
};

// ★★★ **T7 / R6 — a per-row DEPENDENT GATE → a `TriState` `Field` over `DEPENDENT_GATES[$idx]`.**
//
// The I-4 emulation, one row deeper than `decl_tristate!`: the frozen seam's `Field.live` takes no
// row (SPEC_interview.md §10), so `live` is `|_| true` and the ROW's own liveness is expressed by
// `get` returning `None` — which the renderer already treats as hidden — and by `set` refusing
// `NoSuchRow`. Widening the seam to `fn(&ReturnInputs, &RowAddr) → bool` was decided AGAINST in R6:
// 98 mechanical closure edits, a broken freeze, and no behaviour the emulation does not give.
//
// ★★ The closures are NON-CAPTURING (a `const` registry path plus a literal index), which is what
//      lets them coerce to the bare `fn` pointers `Field` requires.
macro_rules! dep_gate_tristate {
    ($idx:literal, $fid:expr) => {
        Field {
            id: $fid,
            label: DEPENDENT_GATES[$idx].prompt,
            help: DEPENDENT_GATES[$idx].help,
            kind: FieldKind::TriState,
            live: |_| true,
            get: |ri, a| {
                if !DEPENDENT_GATES[$idx].live(ri, a.0[0]) {
                    return None;
                }
                Some(FieldValue::TriState((DEPENDENT_GATES[$idx].get)(
                    ri.header.dependents.get(a.0[0])?,
                )))
            },
            set: |ri, a, v| {
                if !DEPENDENT_GATES[$idx].live(ri, a.0[0]) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::TriState(Some(b)) = v else {
                    return Err(SetError::WrongKind);
                };
                (DEPENDENT_GATES[$idx].set)(
                    ri.header
                        .dependents
                        .get_mut(a.0[0])
                        .ok_or(SetError::NoSuchRow)?,
                    b,
                );
                Ok(())
            },
            clear: Some(|ri, a| {
                (DEPENDENT_GATES[$idx].clear)(
                    ri.header
                        .dependents
                        .get_mut(a.0[0])
                        .ok_or(SetError::NoSuchRow)?,
                );
                Ok(())
            }),
        }
    };
}

// ── 5. Dependents (Repeating) — `header.dependents: Vec<Dependent>`, indexed by `addr.0[0]` ─────────────────

const DEPENDENT_FIELDS: &[Field] = &[
    Field {
        id: FieldId::DepName,
        clear: None,
        label: "Dependent name",
        help: "The dependent's full name (1040 dependents grid).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.header
                .dependents
                .get(a.0[0])
                .map(|d| FieldValue::Text(d.name.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header
                .dependents
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepSsn,
        clear: None,
        label: "Dependent SSN",
        help: "The dependent's Social Security number. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, a| {
            ri.header
                .dependents
                .get(a.0[0])
                .map(|d| FieldValue::Secret(mask_ssn(&d.ssn)))
        },
        set: |ri, a, v| {
            let FieldValue::SecretEntry(s) = v else {
                return Err(SetError::WrongKind);
            };
            // ★★★ **R10.3 / fold I10 — THE ROW'S SSN *IS* ITS DILIGENCE IDENTITY.** A changed SSN is a
            //     DIFFERENT PERSON, so the gate answers recorded against the old identity must stop
            //     standing as this child's answers: they move to `answer_log_history` and the new
            //     identity starts with no records at all. Leaving them keyed to the old hash would be
            //     harmless; re-pointing them at the new one would be *"a diligence record that lies"*.
            //
            //     ★ It is done HERE, in the seam's own setter, rather than in `apply`, so every caller
            //       of the form engine gets it — the TUI, a future web renderer, and any test that
            //       drives `Field::set` directly.
            let old = ri
                .header
                .dependents
                .get(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .ssn
                .clone();
            btctax_core::tax::provenance::supersede_dependent_identity(ri, &old, &s);
            ri.header
                .dependents
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepRelationship,
        clear: None,
        label: "Relationship",
        help: "The dependent's relationship to you (e.g. son, daughter, parent).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.header
                .dependents
                .get(a.0[0])
                .map(|d| FieldValue::Text(d.relationship.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header
                .dependents
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .relationship = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepDob,
        clear: None,
        label: "Date of birth",
        help: "The dependent's date of birth.",
        kind: FieldKind::Date,
        live: |_| true,
        get: |ri, a| {
            ri.header
                .dependents
                .get(a.0[0])
                .map(|d| FieldValue::Date(d.date_of_birth))
        },
        set: |ri, a, v| {
            let FieldValue::Date(d) = v else {
                return Err(SetError::WrongKind);
            };
            ri.header
                .dependents
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .date_of_birth = d;
            Ok(())
        },
    },
    // ── ★★★ T7 / R6 — the twenty §152 gates, in `DEPENDENT_GATES` order. ──
    dep_gate_tristate!(1, FieldId::DepGateQcRelationship),
    dep_gate_tristate!(2, FieldId::DepGateYoungerThanYouOrSpouse),
    dep_gate_tristate!(3, FieldId::DepGateFullTimeStudent),
    dep_gate_tristate!(4, FieldId::DepGatePermanentlyAndTotallyDisabled),
    dep_gate_tristate!(5, FieldId::DepGateProvidedOverHalfOwnSupport),
    dep_gate_tristate!(6, FieldId::DepGateFilingJointReturn),
    dep_gate_tristate!(7, FieldId::DepGateJointReturnOnlyToClaimRefund),
    dep_gate_tristate!(8, FieldId::DepGateLivedWithYouOverHalfYear),
    dep_gate_tristate!(9, FieldId::DepGateLivedWithYouInUs),
    dep_gate_tristate!(10, FieldId::DepGateQualifyingChildOfAnotherPerson),
    dep_gate_tristate!(11, FieldId::DepGateCitizenNationalResidentOrCanadaMexico),
    dep_gate_tristate!(12, FieldId::DepGateMarried),
    dep_gate_tristate!(13, FieldId::DepGateTinIssuedByDueDate),
    dep_gate_tristate!(14, FieldId::DepGateCitizenNationalOrResidentAlien),
    dep_gate_tristate!(15, FieldId::DepGateSsnsValidForEmploymentIssuedByDueDate),
    dep_gate_tristate!(16, FieldId::DepGateQrRelationshipOrMemberOfHousehold),
    dep_gate_tristate!(17, FieldId::DepGateQualifyingChildOfAnyTaxpayer),
    dep_gate_tristate!(18, FieldId::DepGateGrossIncomeUnderLimit),
    dep_gate_tristate!(19, FieldId::DepGateYouProvidedOverHalfSupport),
    dep_gate_tristate!(
        20,
        FieldId::DepGateDivorcedSeparatedMultipleSupportOrKidnappedRuleApplies
    ),
];

pub(crate) const DEPENDENTS: Section = Section {
    id: SectionId::Dependents,
    title: "Dependents",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.header.dependents.len(),
        // Top-level Vec: `add` always has a container, so it succeeds; `remove` reports out-of-range (I-4).
        add: |ri, _| {
            ri.header.dependents.push(Dependent::default());
            Ok(())
        },
        remove: |ri, a| {
            if a.0[0] < ri.header.dependents.len() {
                // ★★★ **R10.3 — `remove` DELETES THAT IDENTITY'S ENTRIES.** Without this, deleting
                //     row 0 would leave its gate answers in the log with nobody to attach them to,
                //     and the next row to be given that SSN would inherit another child's
                //     `answered_on` and `prompt_hash`. Keyed by `ssn_hash`, so the OTHER rows'
                //     records are untouched — which is exactly what the row-index key could not do.
                let ssn = ri.header.dependents[a.0[0]].ssn.clone();
                btctax_core::tax::provenance::retire_dependent_identity(ri, &ssn);
                ri.header.dependents.remove(a.0[0]);
                Ok(())
            } else {
                Err(SetError::NoSuchRow)
            }
        },
    },
    fields: DEPENDENT_FIELDS,
};

// ── 6. W2s (Repeating) — `ri.w2s: Vec<W2>`, indexed by `addr.0[0]` ──────────────────────────────────────────

const W2_FIELDS: &[Field] = &[
    Field {
        id: FieldId::W2Owner,
        clear: None,
        label: "Owner",
        help: "Whose W-2 this is (Taxpayer or Spouse) — load-bearing for the per-earner SS wage cap (§1402(b)).",
        kind: FieldKind::Enum(&["Taxpayer", "Spouse"]),
        live: |_| true,
        get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Choice(format!("{:?}", w.owner))),
        set: |ri, a, v| {
            let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
            let owner = match c.as_str() {
                "Taxpayer" => btctax_core::tax::return_inputs::Owner::Taxpayer,
                "Spouse" => btctax_core::tax::return_inputs::Owner::Spouse,
                _ => return Err(SetError::WrongKind),
            };
            ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.owner = owner;
            Ok(())
        },
    },
    Field {
        id: FieldId::W2Employer,
        clear: None,
        label: "c Employer's name, address, and ZIP code",
        help: "Box c \u{201c}Employer's name, address, and ZIP code\u{201d} \u{2014} the employer's \
               name, which is what btctax stores; the address is printed on the W-2 you keep, and no \
               line of the federal return reads it.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Text(w.employer.clone())),
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.employer = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::W2Ein,
        clear: None,
        label: "b Employer identification number (EIN)",
        help: "Box b \u{201c}Employer identification number (EIN)\u{201d}. Required only when Social \
               Security withheld exceeds the §3101(a) cap: the excess-SS credit needs MORE THAN ONE \
               EMPLOYER (Schedule 3 line 11), and one employer's over-withholding is recovered from \
               the employer, never on the return.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.w2s
                .get(a.0[0])
                .map(|w| FieldValue::Text(w.ein.clone().unwrap_or_default()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            let w = ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?;
            // Blank means NOT STATED, which is different from "stated as empty" — the refusal screen
            // distinguishes them and asks rather than guessing.
            w.ein = Some(s).filter(|t| !t.trim().is_empty());
            Ok(())
        },
    },
    // ★★★ R4 / T5 — THE LABELS ARE THE BOX CAPTIONS, VERBATIM. `xtask box-census` joins each
    //     printed box to the field below and asserts the box's own words appear in its label or
    //     help, so a caption a revision re-words cannot keep pointing at a field describing the old
    //     one. The reach stays in the help, where a reader needs it.
    w2_money!(FieldId::Box1Wages, "1 Wages, tips, other compensation", "Box 1 \u{201c}Wages, tips, other compensation\u{201d} \u{2014} Form 1040 line 1a.", box1_wages),
    w2_money!(FieldId::Box2FedWh, "2 Federal income tax withheld", "Box 2 \u{201c}Federal income tax withheld\u{201d} \u{2014} Form 1040 line 25a.", box2_fed_withheld),
    w2_money!(FieldId::Box3SsWages, "3 Social security wages", "Box 3 \u{201c}Social security wages\u{201d} \u{2014} the per-earner §3101(a) cap and the excess-SS credit.", box3_ss_wages),
    w2_money!(FieldId::Box4SsWh, "4 Social security tax withheld", "Box 4 \u{201c}Social security tax withheld\u{201d} \u{2014} Schedule 3 line 11, the §6413(c) excess-SS credit.", box4_ss_withheld),
    w2_money!(FieldId::Box5MedWages, "5 Medicare wages and tips", "Box 5 \u{201c}Medicare wages and tips\u{201d} \u{2014} Form 8959 Part I (Additional Medicare Tax).", box5_medicare_wages),
    w2_money!(FieldId::Box6MedWh, "6 Medicare tax withheld", "Box 6 \u{201c}Medicare tax withheld\u{201d} \u{2014} Form 8959 Part V \u{2192} Form 1040 line 25c.", box6_medicare_withheld),
    w2_money!(FieldId::Box7SsTips, "7 Social security tips", "Box 7 \u{201c}Social security tips\u{201d} \u{2014} the §6413(c) wage total, and the starting point for Schedule 1-A line 4a's qualified tips.", box7_ss_tips),
    w2_money!(FieldId::Box17StateWh, "17 State income tax", "Box 17 \u{201c}State income tax\u{201d} \u{2014} Schedule A line 5a, on the income-tax election.", box17_state_tax_withheld),
    w2_money!(FieldId::Box19LocalTax, "19 Local income tax", "Box 19 \u{201c}Local income tax\u{201d} \u{2014} Schedule A line 5a.", box19_local_tax),
    w2_money!(FieldId::Box8AllocTips, "8 Allocated tips", "Box 8 \u{201c}Allocated tips\u{201d} \u{2014} unreported tip income needing Form 4137, which btctax does not build, so any amount refuses.", box8_allocated_tips),
    w2_money!(FieldId::Box10DepCare, "10 Dependent care benefits", "Box 10 \u{201c}Dependent care benefits\u{201d} \u{2014} needs Form 2441, which btctax does not build, so any amount refuses.", box10_dependent_care),
    Field {
        id: FieldId::W2Box13StatutoryEmployee,
        clear: None,
        label: "13 Statutory employee",
        help: "Check this ONLY if the \"Statutory employee\" box on your Form W-2 is checked. A \
               checked box 13 means this W-2's box-1 wages are business receipts: they belong on \
               SCHEDULE C LINE 1, with the expenses of earning them deducted against them, not on \
               Form 1040 line 1a. btctax files every W-2's box 1 on line 1a, so a checked box \
               refuses rather than file the wages on the wrong line.",
        kind: FieldKind::Bool,
        live: |_| true,
        get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Bool(w.box13_statutory_employee)),
        set: |ri, a, v| {
            let FieldValue::Bool(b) = v else { return Err(SetError::WrongKind) };
            ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.box13_statutory_employee = b;
            Ok(())
        },
    },
    Field {
        id: FieldId::W2Box14bTtoc,
        clear: None,
        label: "14b Treasury Tipped Occupation Code(s)",
        help: "The code(s) printed in box 14b of a 2026 or later Form W-2 — blank on earlier \
               editions, which print box 14 alone. It is your employer's statement of the \
               occupation your tips were earned in, and Schedule 1-A Part II's Caution turns on it: \
               \"These tips must have been received in an occupation listed at \
               IRS.gov/TippedOccupations.\" Enter it exactly as printed (e.g. 102 for wait staff).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.w2s
                .get(a.0[0])
                .map(|w| FieldValue::Text(w.box14b_treasury_tipped_occupation_codes.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(t) = v else { return Err(SetError::WrongKind) };
            ri.w2s
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .box14b_treasury_tipped_occupation_codes = t;
            Ok(())
        },
    },
];

pub(crate) const W2S: Section = Section {
    id: SectionId::W2s,
    title: "W-2s",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.w2s.len(),
        add: |ri, _| {
            ri.w2s.push(W2::default());
            Ok(())
        },
        remove: |ri, a| {
            if a.0[0] < ri.w2s.len() {
                ri.w2s.remove(a.0[0]);
                Ok(())
            } else {
                Err(SetError::NoSuchRow)
            }
        },
    },
    fields: W2_FIELDS,
};

// ── 7. W2Box12 (Repeating, NESTED) — `ri.w2s[addr.0[0]].box12[addr.0[1]]` ───────────────────────────────────
// Parent address is `[w2_i]`; a row address is `[w2_i, box12_i]`.

const W2_BOX12_FIELDS: &[Field] = &[
    Field {
        id: FieldId::Box12Code,
        clear: None,
        label: "Box 12 — code",
        help: "The W-2 box-12 code letter (e.g. D, DD, W). Only inert-allowlist codes are ignorable (§4.10).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.w2s
                .get(a.0[0])
                .and_then(|w| w.box12.get(a.0[1]))
                .map(|e| FieldValue::Text(e.code.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.w2s
                .get_mut(a.0[0])
                .and_then(|w| w.box12.get_mut(a.0[1]))
                .ok_or(SetError::NoSuchRow)?
                .code = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::Box12Amount,
        clear: None,
        label: "Box 12 — amount",
        help: "The dollars coded on this box-12 line.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, a| {
            ri.w2s
                .get(a.0[0])
                .and_then(|w| w.box12.get(a.0[1]))
                .map(|e| FieldValue::Money(e.amount))
        },
        set: |ri, a, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.w2s
                .get_mut(a.0[0])
                .and_then(|w| w.box12.get_mut(a.0[1]))
                .ok_or(SetError::NoSuchRow)?
                .amount = m;
            Ok(())
        },
    },
];

pub(crate) const W2_BOX12: Section = Section {
    id: SectionId::W2Box12,
    title: "W-2 box 12",
    kind: SectionKind::Repeating {
        len: |ri, a| ri.w2s.get(a.0[0]).map_or(0, |w| w.box12.len()),
        add: |ri, a| {
            // Nested: an absent parent W-2 (`a.0[0]` out of range) → `NoSuchRow`, not a silent no-op (I-4).
            let w = ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?;
            // `Box12Entry` has no `Default` (a blank code + zero dollars is the empty new row).
            w.box12.push(Box12Entry {
                code: String::new(),
                amount: Usd::ZERO,
            });
            Ok(())
        },
        remove: |ri, a| {
            let w = ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?;
            if a.0[1] < w.box12.len() {
                w.box12.remove(a.0[1]);
                Ok(())
            } else {
                Err(SetError::NoSuchRow)
            }
        },
    },
    fields: W2_BOX12_FIELDS,
};

// ── 8. ScheduleA (OptionalSingleton) — 7 money leaves + the 2 registry-driven tri-states ────────────────────

const SCHEDULE_A_FIELDS: &[Field] = &[
    scha_money!(
        FieldId::SaMedical,
        "Medical expenses",
        "Sch A line 1 — medical/dental (§213 7.5% floor).",
        medical
    ),
    scha_money!(
        FieldId::SaSaltRealEstate,
        "Real-estate taxes",
        "Sch A line 5b — real-estate taxes.",
        salt_real_estate
    ),
    scha_money!(
        FieldId::SaSaltPersonalProp,
        "Personal-property taxes",
        "Sch A line 5c — personal-property taxes.",
        salt_personal_property
    ),
    scha_money!(
        FieldId::SaSaltStateEst,
        "State estimated payments",
        "State/local income-tax estimated payments (income-tax path, line 5a).",
        salt_state_estimated_payments
    ),
    scha_money!(
        FieldId::SaSaltPriorYear,
        "Prior-year balance paid",
        "State/local income tax — prior-year balance paid this year (income-tax path).",
        salt_prior_year_balance_paid
    ),
    scha_money!(
        FieldId::SaSaltSalesTaxAmt,
        "General sales-tax amount",
        "Sch A line 5a sales-tax amount — used iff the §164(b)(5) sales-tax election is Yes.",
        salt_sales_tax_amount
    ),
    scha_money!(
        FieldId::SaMortgage1098,
        "Home-mortgage interest (1098)",
        "Sch A line 8a — mortgage interest reported on Form 1098.",
        mortgage_interest_1098
    ),
    scha_money!(
        FieldId::SaInvestmentInterest,
        "Investment interest (§163(d))",
        "Sch A line 9 — \"Investment interest. Attach Form 4952 if required.\" Interest on money you \
         borrowed that is allocable to property held for investment. btctax fills no Form 4952, so \
         this is deductible in full only under i4952's own exception; above it the return refuses.",
        investment_interest
    ),
    // ★★ Form 8960 line 9b. An `Option<Usd>` on `ReturnInputs` — NOT a `ScheduleAInputs` leaf — so it
    //    needs its own `get`/`set` rather than `scha_money!`, and a dedicated `clear` for the same
    //    reason `QbiW2Wages` has one: the generic un-answer path writes `Money(Usd::ZERO)`, which here
    //    would turn "I claimed nothing" into the sworn statement "my allocable state income tax is
    //    zero" — a printed 0 on a signed return instead of a blank line.
    Field {
        id: FieldId::Nii8960Line9b,
        clear: Some(|ri, _| {
            ri.form_8960_line9b = None;
            Ok(())
        }),
        label: "State/local income tax allocable to investment income (Form 8960 line 9b)",
        help: "\"State, local, and foreign income tax.\" If you owe the 3.8% net investment income tax \
               (§1411), the part of the state and local income tax on your Schedule A that is \
               attributable to your investment income is deductible against that income. YOU choose \
               how to split it — the Instructions for Form 8960 say you may use \"any reasonable \
               method\", and one they give is the deducted tax times the ratio of Form 8960 line 8 to \
               your AGI. Leave it blank to claim nothing. The most you may enter is what your return \
               actually deducted after the $10,000 ($5,000 married filing separately) §164(b)(6) cap; \
               it is $0 if you took the standard deduction, and $0 if you elected general SALES taxes \
               (they are never deductible against investment income).",
        kind: FieldKind::Money,
        live: |ri| ri.schedule_a.is_some(),
        get: |ri, _| ri.form_8960_line9b.map(FieldValue::Money),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.form_8960_line9b = Some(m);
            Ok(())
        },
    },
    // ★ Registry-driven — DELEGATES to `SKIPPABLE_QUESTIONS::SalesTaxElection` (index 2). live = schedule_a.is_some().
    skippable_tristate!(2, FieldId::SaSaltUseSalesTax, |ri| {
        if let Some(a) = ri.schedule_a.as_mut() {
            a.salt_use_sales_tax = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // ★ Registry-driven — DELEGATES to `FORM_QUESTIONS::MortgageAllUsedToBuyBuildImprove` (index 7).
    decl_tristate!(7, FieldId::SaMortgageAllUsed, |ri| {
        if let Some(a) = ri.schedule_a.as_mut() {
            a.mortgage_all_used_to_buy_build_improve = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
    // ★ Registry-driven — DELEGATES to `FORM_QUESTIONS::MortgageWithinDebtLimit` (index 13, appended
    //   at the END of that array: `decl_tristate!` couples to the INDEX).
    decl_tristate!(13, FieldId::SaMortgageWithinDebtLimit, |ri| {
        if let Some(a) = ri.schedule_a.as_mut() {
            a.mortgage_within_debt_limit = None;
            Ok(())
        } else {
            Err(SetError::NoSuchRow)
        }
    }),
];

pub(crate) const SCHEDULE_A: Section = Section {
    id: SectionId::ScheduleA,
    title: "Schedule A (itemized deductions)",
    kind: SectionKind::OptionalSingleton {
        present: |ri| ri.schedule_a.is_some(),
        create: |ri| {
            if ri.schedule_a.is_none() {
                ri.schedule_a = Some(ScheduleAInputs::default());
            }
        },
        // ★ I-10 (spec §5.1): deleting Schedule A must clear a `ForceItemize` election back to `Auto`, else a
        // return with no Schedule A would still force itemizing (understating the standard deduction).
        delete: |ri| {
            ri.schedule_a = None;
            ri.itemize_election = ItemizeElection::Auto;
        },
    },
    fields: SCHEDULE_A_FIELDS,
};

// ── 9. ScheduleACharitable (Repeating, NESTED under schedule_a) — `schedule_a.charitable[addr.0[0]]` ─────────

const CHARITABLE_FIELDS: &[Field] = &[
    Field {
        id: FieldId::CharClass,
        clear: None,
        label: "Gift class",
        help: "§170(b) ceiling class: Cash60 / Cash30 / CapGainProp30 / CapGainProp20 / OrdinaryProp50 / OrdinaryProp30.",
        kind: FieldKind::Enum(&[
            "Cash60",
            "Cash30",
            "CapGainProp30",
            "CapGainProp20",
            "OrdinaryProp50",
            "OrdinaryProp30",
        ]),
        live: |_| true,
        get: |ri, a| {
            ri.schedule_a
                .as_ref()
                .and_then(|sa| sa.charitable.get(a.0[0]))
                .map(|g| FieldValue::Choice(format!("{:?}", g.class)))
        },
        set: |ri, a, v| {
            let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
            let class = match c.as_str() {
                "Cash60" => CharitableClass::Cash60,
                "Cash30" => CharitableClass::Cash30,
                "CapGainProp30" => CharitableClass::CapGainProp30,
                "CapGainProp20" => CharitableClass::CapGainProp20,
                "OrdinaryProp50" => CharitableClass::OrdinaryProp50,
                "OrdinaryProp30" => CharitableClass::OrdinaryProp30,
                _ => return Err(SetError::WrongKind),
            };
            ri.schedule_a
                .as_mut()
                .and_then(|sa| sa.charitable.get_mut(a.0[0]))
                .ok_or(SetError::NoSuchRow)?
                .class = class;
            Ok(())
        },
    },
    Field {
        id: FieldId::CharAmount,
        clear: None,
        label: "Gift amount",
        help: "The dollar amount of this non-crypto charitable gift (crypto flows from the ledger).",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, a| {
            ri.schedule_a
                .as_ref()
                .and_then(|sa| sa.charitable.get(a.0[0]))
                .map(|g| FieldValue::Money(g.amount))
        },
        set: |ri, a, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.schedule_a
                .as_mut()
                .and_then(|sa| sa.charitable.get_mut(a.0[0]))
                .ok_or(SetError::NoSuchRow)?
                .amount = m;
            Ok(())
        },
    },
];

pub(crate) const SCHEDULE_A_CHARITABLE: Section = Section {
    id: SectionId::ScheduleACharitable,
    title: "Schedule A — charitable gifts",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.schedule_a.as_ref().map_or(0, |sa| sa.charitable.len()),
        add: |ri, _| {
            // Nested under the optional Schedule A: absent `schedule_a` → `NoSuchRow`, not a no-op (I-4).
            let sa = ri.schedule_a.as_mut().ok_or(SetError::NoSuchRow)?;
            // `CharitableGift`/`CharitableClass` have no `Default`; a cash gift to a 50%-org is the
            // most common class and a safe starting point (the filer then picks the real class).
            sa.charitable.push(CharitableGift {
                class: CharitableClass::Cash60,
                amount: Usd::ZERO,
            });
            Ok(())
        },
        remove: |ri, a| {
            let sa = ri.schedule_a.as_mut().ok_or(SetError::NoSuchRow)?;
            if a.0[0] < sa.charitable.len() {
                sa.charitable.remove(a.0[0]);
                Ok(())
            } else {
                Err(SetError::NoSuchRow)
            }
        },
    },
    fields: CHARITABLE_FIELDS,
};

// ── 10. Payments (Singleton) — `ri.payments` ────────────────────────────────────────────────────────────────

const PAYMENTS_FIELDS: &[Field] = &[
    Field {
        id: FieldId::PayEstimated,
        clear: None,
        label: "Estimated tax payments",
        help: "§6654 estimated-tax payments made for the year → 1040 line 26.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.estimated_tax_payments)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.payments.estimated_tax_payments = m;
            Ok(())
        },
    },
    Field {
        id: FieldId::PayExtension,
        clear: None,
        label: "Extension payment",
        help: "Amount paid with a Form 4868 extension request → Sch 3 line 10.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.extension_payment)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.payments.extension_payment = m;
            Ok(())
        },
    },
    Field {
        id: FieldId::PayOtherWh,
        clear: None,
        label: "Other withholding",
        help:
            "Other federal income tax withheld (e.g. Form 1099 backup withholding) → 1040 line 25c.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.other_withholding)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.payments.other_withholding = m;
            Ok(())
        },
    },
];

/// ★★ §G-22 — the two QBI loss carryforwards (Form 8995 lines **7** and **3**).
///
/// **Every carryforward family was import-only**, reachable solely by hand-editing the TOML: capital
/// loss, charitable, and both of these. For the first two that is a conservative omission — a forgone
/// benefit that OVERSTATES tax. **These two are the opposite.** They are prior-year LOSSES that reduce
/// the §199A deduction, so leaving them at zero INFLATES the deduction and **UNDERSTATES the tax**.
///
/// ★ Line 7's hole shipped in v0.14.0; line 3's was created and closed in the same branch.
const CARRYFORWARD_FIELDS: &[Field] = &[
    Field {
        id: FieldId::QbiReitPtpCarryforwardIn,
        clear: None,
        label: "Prior-year qualified REIT dividend / PTP LOSS carryforward (Form 8995 line 7)",
        help: "A POSITIVE amount. It reduces this year's REIT/PTP income at line 8, so leaving it out \
               inflates your §199A deduction and understates your tax. Enter it from line 17 of last \
               year's Form 8995. Leave 0 if you had none.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.qbi.reit_ptp_carryforward_in)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.qbi.reit_ptp_carryforward_in = m;
            // ★ A figure the FILER typed is theirs, not ours: stamping it `User` is what makes the
            // write-back refuse to overwrite it without `--force`.
            ri.qbi.reit_ptp_carryforward_in_provenance = btctax_core::tax::return_inputs::CarryProvenance::User;
            Ok(())
        },
    },
    Field {
        id: FieldId::QbiCarryforwardIn,
        clear: None,
        label: "Prior-year qualified business net LOSS carryforward (Form 8995 line 3)",
        help: "A POSITIVE amount. It is subtracted at line 4, so leaving it out inflates your §199A \
               deduction and understates your tax. Enter it from line 16 of last year's Form 8995. \
               Leave 0 if you had none.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.qbi.qbi_carryforward_in)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.qbi.qbi_carryforward_in = m;
            ri.qbi.qbi_carryforward_in_provenance = btctax_core::tax::return_inputs::CarryProvenance::User;
            Ok(())
        },
    },
];

/// §G-28/B1b — Form 8995-A Part II's two inputs. Live only when there IS a trade or business.
const QBI_LIMITATION_FIELDS: &[Field] = &[
    Field {
        id: FieldId::QbiW2Wages,
        // ★★★ A DEDICATED `clear`, and it is load-bearing. These two — and `Nii8960Line9b`, which
        //     carries its own `clear` for the same reason — are the `Option<Usd>` leaves in
        //     `ReturnInputs`, the fields for which the generic un-answer path —
        //     `set(empty_for_kind)`, which for `FieldKind::Money` is `Money(Usd::ZERO)` — would write
        //     `Some($0)` instead of `None`. That is not "cleared": it is the answer "my business paid
        //     no wages", and it is EXACTLY the state `screen_absolute`'s `QbiAboveThreshold` refusal
        //     tests for. Laundering it defeats the only guard standing in front of
        //     `qbi_w2_wages.unwrap_or(ZERO)`, and caps a wage-paying filer's deduction at zero —
        //     OVERSTATING their tax.
        clear: Some(|ri, _| {
            ri.schedule_c
                .as_mut()
                .ok_or(SetError::NoSuchRow)?
                .qbi_w2_wages = None;
            Ok(())
        }),
        label: "W-2 wages your business paid (Form 8995-A line 4)",
        help: "\"Allocable share of W-2 wages from the trade, business, or aggregation.\" Needed only                if your taxable income is above $191,950 ($383,900 married filing jointly) — above that,                §199A(b)(2) caps your deduction at the greater of 50% of these wages, or 25% of them                plus 2.5% of your qualified-property basis. A sole proprietor with NO employees enters                0, and that is a real answer: it caps the deduction at zero.",
        kind: FieldKind::Money,
        live: |ri| ri.schedule_c.is_some(),
        get: |ri, _| {
            ri.schedule_c
                .as_ref()
                .and_then(|c| c.qbi_w2_wages)
                .map(FieldValue::Money)
        },
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.schedule_c
                .as_mut()
                .ok_or(SetError::NoSuchRow)?
                .qbi_w2_wages = Some(m);
            Ok(())
        },
    },
    Field {
        id: FieldId::QbiUbia,
        // ★★★ Same reason as `QbiW2Wages` above — see that comment.
        clear: Some(|ri, _| {
            ri.schedule_c
                .as_mut()
                .ok_or(SetError::NoSuchRow)?
                .qbi_ubia = None;
            Ok(())
        }),
        label: "Unadjusted basis of qualified property (UBIA) (Form 8995-A line 7)",
        help: "\"Allocable share of the unadjusted basis immediately after acquisition (UBIA) of all                qualified property.\" The original cost of depreciable property the business still                uses, before depreciation. Needed only above the taxable-income threshold; enter 0 if                the business holds no such property.",
        kind: FieldKind::Money,
        live: |ri| ri.schedule_c.is_some(),
        get: |ri, _| {
            ri.schedule_c
                .as_ref()
                .and_then(|c| c.qbi_ubia)
                .map(FieldValue::Money)
        },
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else {
                return Err(SetError::WrongKind);
            };
            ri.schedule_c.as_mut().ok_or(SetError::NoSuchRow)?.qbi_ubia = Some(m);
            Ok(())
        },
    },
];

pub(crate) const QBI_LIMITATION: Section = Section {
    id: SectionId::QbiLimitation,
    title: "§199A limitation (only if your income is above the threshold)",
    kind: SectionKind::Singleton,
    fields: QBI_LIMITATION_FIELDS,
};

pub(crate) const CARRYFORWARDS: Section = Section {
    id: SectionId::Carryforwards,
    title: "Carryforwards from last year",
    kind: SectionKind::Singleton,
    fields: CARRYFORWARD_FIELDS,
};

pub(crate) const PAYMENTS: Section = Section {
    id: SectionId::Payments,
    title: "Payments",
    kind: SectionKind::Singleton,
    fields: PAYMENTS_FIELDS,
};

#[cfg(test)]
mod tests {
    use super::super::{fresh_single, section};
    use crate::seam::{
        FieldId, FieldKind, FieldValue, RowAddr, SecretView, SectionId, SectionKind, SetError,
    };
    use btctax_core::tax::return_inputs::ItemizeElection;
    use btctax_core::tax::types::FilingStatus;
    use rust_decimal_macros::dec;

    #[test]
    fn w2_repeating_with_nested_box12_reads_and_writes() {
        let mut ri = fresh_single();
        let w2s = section(SectionId::W2s);
        let SectionKind::Repeating { add, len, .. } = w2s.kind else {
            panic!()
        };
        (add)(&mut ri, &RowAddr::default()).unwrap();
        assert_eq!((len)(&ri, &RowAddr::default()), 1);
        let box1 = w2s
            .fields
            .iter()
            .find(|f| f.id == FieldId::Box1Wages)
            .unwrap();
        (box1.set)(&mut ri, &RowAddr(vec![0]), FieldValue::Money(dec!(50000))).unwrap();
        assert_eq!(ri.w2s[0].box1_wages, dec!(50000));
        // nested box12
        let b12 = section(SectionId::W2Box12);
        let SectionKind::Repeating { add: add12, .. } = b12.kind else {
            panic!()
        };
        (add12)(&mut ri, &RowAddr(vec![0])).unwrap(); // parent = w2 index 0
        assert_eq!(ri.w2s[0].box12.len(), 1);
        let amt = b12
            .fields
            .iter()
            .find(|f| f.id == FieldId::Box12Amount)
            .unwrap();
        (amt.set)(
            &mut ri,
            &RowAddr(vec![0, 0]),
            FieldValue::Money(dec!(23000)),
        )
        .unwrap();
        assert_eq!(ri.w2s[0].box12[0].amount, dec!(23000));
    }

    #[test]
    fn schedule_a_optional_singleton_create_delete_resets_itemize_election() {
        let mut ri = fresh_single();
        ri.itemize_election = ItemizeElection::ForceItemize;
        let sa = section(SectionId::ScheduleA);
        let SectionKind::OptionalSingleton {
            create,
            delete,
            present,
        } = sa.kind
        else {
            panic!()
        };
        (create)(&mut ri);
        assert!((present)(&ri));
        (delete)(&mut ri);
        assert!(!(present)(&ri));
        assert_eq!(
            ri.itemize_election,
            ItemizeElection::Auto,
            "I-10: delete resets ForceItemize"
        );
    }

    #[test]
    fn itemize_election_live_only_with_schedule_a_i5() {
        // ★ I-5 (spec §5.8): the election is live ONLY with a Schedule A. This pins the liveness metadata so
        // a mutation back to `|_| true` cannot silently re-open the $0-deduction ForceItemize hazard.
        let opts = section(SectionId::ReturnOptions);
        let f = opts
            .fields
            .iter()
            .find(|f| f.id == FieldId::ItemizeElection)
            .unwrap();
        let mut ri = fresh_single();
        assert!(
            !(f.live)(&ri),
            "I-5: ItemizeElection not live without a Schedule A"
        );
        let sa = section(SectionId::ScheduleA);
        let SectionKind::OptionalSingleton { create, .. } = sa.kind else {
            panic!()
        };
        (create)(&mut ri);
        assert!(
            (f.live)(&ri),
            "I-5: ItemizeElection live once a Schedule A exists"
        );
    }

    #[test]
    fn singleton_and_optional_singleton_get_set_spotcheck() {
        let mut ri = fresh_single();
        // ReturnOptions singleton: FilingStatus enum roundtrip + wrong-choice rejection.
        let ro = section(SectionId::ReturnOptions);
        let fs = ro
            .fields
            .iter()
            .find(|f| f.id == FieldId::FilingStatus)
            .unwrap();
        assert_eq!(
            (fs.get)(&ri, &RowAddr::default()),
            Some(FieldValue::Choice("Single".into()))
        );
        (fs.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::Choice("Mfj".into()),
        )
        .unwrap();
        assert_eq!(ri.filing_status, FilingStatus::Mfj);
        assert_eq!(
            (fs.set)(
                &mut ri,
                &RowAddr::default(),
                FieldValue::Choice("Nope".into())
            ),
            Err(SetError::WrongKind)
        );

        // Payments singleton money.
        let pay = section(SectionId::Payments);
        let est = pay
            .fields
            .iter()
            .find(|f| f.id == FieldId::PayEstimated)
            .unwrap();
        (est.set)(&mut ri, &RowAddr::default(), FieldValue::Money(dec!(1200))).unwrap();
        assert_eq!(ri.payments.estimated_tax_payments, dec!(1200));

        // Spouse optional-singleton: get None + set NoSuchRow until created.
        let sp = section(SectionId::Spouse);
        let SectionKind::OptionalSingleton {
            create, present, ..
        } = sp.kind
        else {
            panic!()
        };
        let sp_first = sp
            .fields
            .iter()
            .find(|f| f.id == FieldId::SpFirstName)
            .unwrap();
        assert_eq!(
            (sp_first.get)(&ri, &RowAddr::default()),
            None,
            "Sp* get is None without a spouse"
        );
        assert_eq!(
            (sp_first.set)(&mut ri, &RowAddr::default(), FieldValue::Text("Pat".into())),
            Err(SetError::NoSuchRow),
            "Sp* set refuses without a spouse"
        );
        assert!(!(present)(&ri));
        (create)(&mut ri);
        assert!((present)(&ri));
        (sp_first.set)(&mut ri, &RowAddr::default(), FieldValue::Text("Pat".into())).unwrap();
        assert_eq!(
            (sp_first.get)(&ri, &RowAddr::default()),
            Some(FieldValue::Text("Pat".into()))
        );
    }

    #[test]
    fn secret_fields_mask_and_never_leak_digits() {
        let mut ri = fresh_single();
        let tp = section(SectionId::Taxpayer);
        let ssn = tp.fields.iter().find(|f| f.id == FieldId::TpSsn).unwrap();
        // Empty storage → Empty view.
        assert_eq!(
            (ssn.get)(&ri, &RowAddr::default()),
            Some(FieldValue::Secret(SecretView::Empty))
        );
        // Raw entry is stored verbatim (canonicalization is Task 8's parse); get masks it.
        (ssn.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::SecretEntry("123456789".into()),
        )
        .unwrap();
        assert_eq!(ri.header.taxpayer.ssn, "123456789");
        let FieldValue::Secret(SecretView::Set { masked }) =
            (ssn.get)(&ri, &RowAddr::default()).unwrap()
        else {
            panic!("expected a Set secret view")
        };
        assert_eq!(masked, "***-**-6789");
        assert!(
            !masked.contains("12345"),
            "masked must not leak leading digits"
        );
        // A Secret set rejects a non-SecretEntry value.
        assert_eq!(
            (ssn.set)(&mut ri, &RowAddr::default(), FieldValue::Text("x".into())),
            Err(SetError::WrongKind)
        );

        // IpPin: Option<String>. None → Empty; Some → Set{masked}.
        let ippin = tp.fields.iter().find(|f| f.id == FieldId::IpPin).unwrap();
        assert_eq!(
            (ippin.get)(&ri, &RowAddr::default()),
            Some(FieldValue::Secret(SecretView::Empty))
        );
        (ippin.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::SecretEntry("112233".into()),
        )
        .unwrap();
        assert_eq!(ri.header.ip_pin.as_deref(), Some("112233"));
        let FieldValue::Secret(SecretView::Set { masked }) =
            (ippin.get)(&ri, &RowAddr::default()).unwrap()
        else {
            panic!("expected a Set secret view for ip_pin")
        };
        // ★ I-3 (updated from the old buggy `***-**-2233`): the IP-PIN mask reveals ZERO digits — the
        // anti-fraud discipline of `IpPin(******)`, NOT the SSN's last-4 reveal.
        assert_eq!(masked, "******");
        for d in ['1', '2', '3'] {
            assert!(
                !masked.contains(d),
                "IP-PIN mask must not leak digit {d}: {masked}"
            );
        }

        // ★ I-2: an empty entry (and thus `ClearField`) maps the IP PIN to `None`, never `Some("")`.
        (ippin.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::SecretEntry(String::new()),
        )
        .unwrap();
        assert_eq!(
            ri.header.ip_pin, None,
            "empty IP-PIN entry must clear to None, not Some(\"\")"
        );

        // ★ I-3: a short/non-canonical SSN entry is FULLY masked (no partial reveal — folds follow-up (k)).
        (ssn.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::SecretEntry("12".into()),
        )
        .unwrap();
        let FieldValue::Secret(SecretView::Set { masked }) =
            (ssn.get)(&ri, &RowAddr::default()).unwrap()
        else {
            panic!("expected a Set secret view")
        };
        assert_eq!(
            masked, "***-**-****",
            "a non-9-digit SSN entry is fully masked"
        );
    }

    #[test]
    fn schedule_a_registry_driven_fields_delegate_to_core_registries() {
        use btctax_core::tax::questions::{
            QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
        };
        let sa = section(SectionId::ScheduleA);
        let salt = sa
            .fields
            .iter()
            .find(|f| f.id == FieldId::SaSaltUseSalesTax)
            .unwrap();
        let mortgage = sa
            .fields
            .iter()
            .find(|f| f.id == FieldId::SaMortgageAllUsed)
            .unwrap();
        assert_eq!(salt.kind, FieldKind::TriState);
        assert_eq!(mortgage.kind, FieldKind::TriState);

        // SALT delegates to SKIPPABLE_QUESTIONS::SalesTaxElection.
        let salt_entry = SKIPPABLE_QUESTIONS
            .iter()
            .find(|e| e.id == SkippableId::SalesTaxElection)
            .unwrap();
        let mut ri = fresh_single();
        ri.schedule_a = Some(Default::default());
        (salt.set)(
            &mut ri,
            &RowAddr::default(),
            FieldValue::TriState(Some(true)),
        )
        .unwrap();
        assert_eq!(
            (salt_entry.get_bool)(&ri),
            Some(true),
            "SALT set delegates to the registry"
        );
        assert_eq!(
            (salt.get)(&ri, &RowAddr::default()),
            Some(FieldValue::TriState(Some(true)))
        );
        assert_eq!(
            (salt.live)(&ri),
            (salt_entry.live)(&ri),
            "SALT live comes from the registry"
        );

        // Mortgage delegates to FORM_QUESTIONS::MortgageAllUsedToBuyBuildImprove.
        let m_entry = FORM_QUESTIONS
            .iter()
            .find(|e| e.id == QuestionId::MortgageAllUsedToBuyBuildImprove)
            .unwrap();
        ri.schedule_a.as_mut().unwrap().mortgage_interest_1098 = dec!(1); // make the mortgage question live
        (m_entry.set)(&mut ri, true);
        assert_eq!(
            (mortgage.get)(&ri, &RowAddr::default()),
            Some(FieldValue::TriState(Some(true)))
        );
        assert_eq!(
            (mortgage.live)(&ri),
            (m_entry.live)(&ri),
            "mortgage live comes from the registry gate"
        );
    }
}

// ── spec 1099-DA T6. BrokerReporting (Repeating) — `ri.broker_reporting.0: BTreeMap<provider, CohortAnswers>`,
//    row i = the i-th provider in key order; two Enum slots per row. ──────────────────────────────────────

/// The Enum tokens: `Unanswered` is the ABSENT slot (`None`), the five others are `BrokerReported`'s
/// variants by their Debug names. Choosing `Unanswered` clears the slot, which is the un-answer path.
const BROKER_CHOICES: &[&str] = &[
    "Unanswered",
    "NotReported",
    "ProceedsOnly",
    "BasisMatches",
    "BasisDiffers",
    "Mixed",
];

fn broker_choice(v: Option<BrokerReported>) -> FieldValue {
    FieldValue::Choice(
        match v {
            None => "Unanswered",
            Some(BrokerReported::NotReported) => "NotReported",
            Some(BrokerReported::ProceedsOnly) => "ProceedsOnly",
            Some(BrokerReported::BasisMatches) => "BasisMatches",
            Some(BrokerReported::BasisDiffers) => "BasisDiffers",
            Some(BrokerReported::Mixed) => "Mixed",
        }
        .to_string(),
    )
}

fn broker_parse(v: FieldValue) -> Result<Option<BrokerReported>, SetError> {
    let FieldValue::Choice(c) = v else {
        return Err(SetError::WrongKind);
    };
    Ok(match c.as_str() {
        "Unanswered" => None,
        "NotReported" => Some(BrokerReported::NotReported),
        "ProceedsOnly" => Some(BrokerReported::ProceedsOnly),
        "BasisMatches" => Some(BrokerReported::BasisMatches),
        "BasisDiffers" => Some(BrokerReported::BasisDiffers),
        "Mixed" => Some(BrokerReported::Mixed),
        _ => return Err(SetError::WrongKind),
    })
}

fn broker_get(
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
    a: &crate::seam::RowAddr,
    cohort: Cohort,
) -> Option<FieldValue> {
    ri.broker_reporting
        .0
        .values()
        .nth(a.0[0])
        .map(|c| broker_choice(c.get(cohort)))
}

fn broker_set(
    ri: &mut btctax_core::tax::return_inputs::ReturnInputs,
    a: &crate::seam::RowAddr,
    cohort: Cohort,
    v: FieldValue,
) -> Result<(), SetError> {
    let answer = broker_parse(v)?;
    let slot = ri
        .broker_reporting
        .0
        .values_mut()
        .nth(a.0[0])
        .ok_or(SetError::NoSuchRow)?;
    match cohort {
        Cohort::Covered => slot.covered = answer,
        Cohort::Noncovered => slot.noncovered = answer,
    }
    Ok(())
}

const BROKER_FIELDS: &[Field] = &[
    Field {
        id: FieldId::BrokerCovered,
        clear: None,
        label: "Covered lots — bought on this venue on/after 2026-01-01",
        help: "Look at the Form 1099-DA(s) this venue issued for the rows under this key. None lists \
               them → NotReported (box I/L). All listed with box 2 NOT checked → ProceedsOnly (box \
               H/K). All listed with box 2 checked and box 1g equal to column (e) on each → \
               BasisMatches (box G/J). Any box 1g differs → BasisDiffers (the return REFUSES: it \
               needs the broker's figure in (e) and the correction in (g), a per-lot import). The \
               forms do not all say the same thing → Mixed (REFUSES). Unanswered refuses the return.",
        kind: FieldKind::Enum(BROKER_CHOICES),
        live: |_| true,
        get: |ri, a| broker_get(ri, a, Cohort::Covered),
        set: |ri, a, v| broker_set(ri, a, Cohort::Covered, v),
    },
    Field {
        id: FieldId::BrokerNoncovered,
        clear: None,
        label: "Noncovered lots — everything else this venue sold for you",
        help: "Same five answers, for the rows this venue sold that arrived by transfer, were bought \
               before 2026, or were credited as rewards. A broker reports these with box 2 unchecked \
               (proceeds only) or not at all. Leave Unanswered when the row list shows 0 rows: an \
               answer no row reads refuses as unread.",
        kind: FieldKind::Enum(BROKER_CHOICES),
        live: |_| true,
        get: |ri, a| broker_get(ri, a, Cohort::Noncovered),
        set: |ri, a, v| broker_set(ri, a, Cohort::Noncovered, v),
    },
];

pub(crate) const BROKER_REPORTING: Section = Section {
    id: SectionId::BrokerReporting,
    title: "Form 1099-DA answers",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.broker_reporting.0.len(),
        // ★ Rows are the ledger's exchange keys, seeded by the renderer at open — a row typed by hand
        //   would be an answer no Form 8949 row reads, which the return refuses as unread. So `add`
        //   REPORTS (I-4) rather than inventing a provider name.
        add: |_, _| Err(SetError::Immutable),
        remove: |ri, a| {
            let key = ri.broker_reporting.0.keys().nth(a.0[0]).cloned();
            match key {
                Some(k) => {
                    ri.broker_reporting.0.remove(&k);
                    Ok(())
                }
                None => Err(SetError::NoSuchRow),
            }
        },
    },
    fields: BROKER_FIELDS,
};

#[cfg(test)]
mod broker_block_tests {
    use super::*;
    use crate::seam::RowAddr;
    use btctax_core::forms::CohortAnswers;
    use btctax_core::tax::return_inputs::ReturnInputs;

    fn two_providers() -> ReturnInputs {
        let mut ri = ReturnInputs::default();
        ri.broker_reporting.0.insert(
            "coinbase".into(),
            CohortAnswers {
                covered: Some(BrokerReported::BasisMatches),
                noncovered: None,
            },
        );
        ri.broker_reporting
            .0
            .insert("gemini".into(), CohortAnswers::default());
        ri
    }

    /// spec 1099-DA T6 — each slot reads and writes exactly its (provider, cohort); `Unanswered`
    /// clears the slot (the un-answer path); `add` refuses (rows are the ledger's keys); `remove`
    /// deletes the provider; a bad row or token is a clean error.
    #[test]
    fn the_block_reads_writes_clears_and_refuses_hand_rows() {
        let mut ri = two_providers();
        let (covered, noncovered) = (&BROKER_FIELDS[0], &BROKER_FIELDS[1]);
        let r0 = RowAddr(vec![0]);
        let r1 = RowAddr(vec![1]);
        assert_eq!(
            (covered.get)(&ri, &r0),
            Some(FieldValue::Choice("BasisMatches".into()))
        );
        assert_eq!(
            (noncovered.get)(&ri, &r0),
            Some(FieldValue::Choice("Unanswered".into()))
        );
        (noncovered.set)(&mut ri, &r1, FieldValue::Choice("ProceedsOnly".into())).unwrap();
        assert_eq!(
            ri.broker_reporting.answer("gemini", Cohort::Noncovered),
            Some(BrokerReported::ProceedsOnly)
        );
        assert_eq!(
            ri.broker_reporting.answer("coinbase", Cohort::Noncovered),
            None,
            "the other row is untouched"
        );
        (covered.set)(&mut ri, &r0, FieldValue::Choice("Unanswered".into())).unwrap();
        assert_eq!(
            ri.broker_reporting.answer("coinbase", Cohort::Covered),
            None,
            "cleared"
        );
        assert_eq!(
            (covered.set)(
                &mut ri,
                &RowAddr(vec![7]),
                FieldValue::Choice("Mixed".into())
            ),
            Err(SetError::NoSuchRow)
        );
        assert_eq!(
            (covered.set)(&mut ri, &r0, FieldValue::Choice("Whatever".into())),
            Err(SetError::WrongKind)
        );
        assert_eq!(
            (covered.set)(&mut ri, &r0, FieldValue::Text("Mixed".into())),
            Err(SetError::WrongKind)
        );
        let SectionKind::Repeating { len, add, remove } = BROKER_REPORTING.kind else {
            panic!("repeating")
        };
        assert_eq!(len(&ri, &RowAddr::default()), 2);
        assert_eq!(add(&mut ri, &RowAddr::default()), Err(SetError::Immutable));
        remove(&mut ri, &r0).unwrap();
        assert_eq!(len(&ri, &RowAddr::default()), 1);
        assert!(
            ri.broker_reporting.0.contains_key("gemini")
                && !ri.broker_reporting.0.contains_key("coinbase")
        );
        assert_eq!(remove(&mut ri, &RowAddr(vec![5])), Err(SetError::NoSuchRow));
    }

    /// ★★★ **I10's KILL, AT THE SEAM — a diligence record never describes the wrong person.**
    ///
    /// Two dependents, gate answers on both. Delete **row 0** and:
    ///   · row 1's records are untouched — the thing a `{ row, gate }` key could not deliver, because
    ///     row 1 becomes row 0 the instant the `Vec` shifts;
    ///   · row 0's are gone — no orphan a future row could inherit.
    /// Then change row 1's `ssn`: a different person, so its records become HISTORY and the new
    /// identity starts with none.
    ///
    /// ★ Driven through `Field::set` / `SectionKind::remove` — the seam itself — not through the core
    ///   helpers, because the guarantee is that the *editor* maintains the log, and a test of the
    ///   helpers alone would pass with the seam never calling them.
    #[test]
    fn removing_a_dependent_row_takes_only_that_identitys_answers_and_a_new_ssn_starts_fresh() {
        use btctax_core::tax::provenance::{
            dependent_ssn_hash, record_answer, AnswerKey, AnswerState, DependentGate,
        };
        let (ssn0, ssn1) = ("111-22-3333", "444-00-6666");
        let key = |ssn: &str| AnswerKey::DependentGate {
            ssn_hash: dependent_ssn_hash(ssn),
            gate: DependentGate::QcRelationship,
        };
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.dependents = vec![
            Dependent {
                ssn: ssn0.into(),
                ..Default::default()
            },
            Dependent {
                ssn: ssn1.into(),
                ..Default::default()
            },
        ];
        for ssn in [ssn0, ssn1] {
            record_answer(
                &mut ri,
                key(ssn),
                "a gate prompt",
                time::macros::date!(2026 - 09 - 01),
                AnswerState::Given,
            );
        }

        let SectionKind::Repeating { remove, .. } = DEPENDENTS.kind else {
            panic!("Dependents is a repeating section")
        };
        remove(&mut ri, &RowAddr(vec![0])).unwrap();
        assert_eq!(ri.header.dependents.len(), 1);
        assert!(
            !ri.answer_log.contains_key(&key(ssn0)),
            "the removed row's records must go with it"
        );
        assert!(
            ri.answer_log.contains_key(&key(ssn1)),
            "the SURVIVING row's records must be untouched — this is the whole reason the key is an \
             identity and not the row index, which just shifted from 1 to 0"
        );
        assert!(
            ri.answer_log_history.is_empty(),
            "a withdrawn row is not a superseded answer"
        );

        // The survivor's SSN is corrected: a different person.
        let ssn_field = DEPENDENT_FIELDS
            .iter()
            .find(|f| f.id == FieldId::DepSsn)
            .unwrap();
        (ssn_field.set)(
            &mut ri,
            &RowAddr(vec![0]),
            FieldValue::SecretEntry("777-00-9999".into()),
        )
        .unwrap();
        assert!(
            ri.answer_log.is_empty(),
            "the old identity's records must stop standing as this row's answers"
        );
        assert_eq!(ri.answer_log_history.len(), 1);
        assert_eq!(ri.answer_log_history[0].0, key(ssn1));
    }
}

// ── ★★★ R4 / T5 — THE INFORMATION-RETURN SECTIONS ────────────────────────────────────────────────
//
// One REPEATING section per supported document type, modelled exactly on `W2s`: per row the payer
// identity (`payer`, `payer_tin`, `transcribed_on`) plus **one `Field` per collected box, named for
// the box, carrying the box's own printed CAPTION verbatim in its help** — the transcription rule
// (`CLAUDE.md`: *"one field per numbered line, named for the line, in the form's own numbering,
// carrying the official instruction text verbatim as its doc comment"*), applied one level down.
//
// ★ The captions are the archived extracts' own text (`design/forms/extract/<stem>--<edition>.txt`),
//   and `xtask box-census` joins each box to the `FieldId` below and CHECKS that the caption's words
//   appear in the field's help. A re-worded box, or a field pointed at another section, reds there.
//
// ★★ Every box is a plain `Usd`, never an `Option<Usd>` — R4: *"a document row's every box is
//    testimony by construction, because the row exists only because the filer declared the document
//    (R3) — a 1099-INT with box 2 = 0 is the DOCUMENT's zero."*

/// A repeating money leaf over `ri.$vec[addr.0[0]].$field`.
macro_rules! doc_money {
    ($id:expr, $vec:ident, $label:literal, $help:expr, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, a| ri.$vec.get(a.0[0]).map(|r| FieldValue::Money(r.$field)),
            set: |ri, a, v| {
                let FieldValue::Money(m) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.$vec.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.$field = m;
                Ok(())
            },
        }
    };
}

/// A repeating text leaf over `ri.$vec[addr.0[0]].$field`.
macro_rules! doc_text {
    ($id:expr, $vec:ident, $label:literal, $help:expr, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Text,
            live: |_| true,
            get: |ri, a| {
                ri.$vec
                    .get(a.0[0])
                    .map(|r| FieldValue::Text(r.$field.clone()))
            },
            set: |ri, a, v| {
                let FieldValue::Text(t) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.$vec.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.$field = t;
                Ok(())
            },
        }
    };
}

/// R10.2 — the repeating `transcribed_on` leaf. `None` is a real answer (*"transcribed without a
/// date"*, printed by the packet manifest), so `clear` writes `None` rather than a sentinel date.
macro_rules! doc_transcribed_on {
    ($id:expr, $vec:ident) => {
        Field {
            id: $id,
            clear: Some(|ri, a| {
                ri.$vec
                    .get_mut(a.0[0])
                    .ok_or(SetError::NoSuchRow)?
                    .transcribed_on = None;
                Ok(())
            }),
            label: "Transcribed on",
            help: "The date you copied this document's boxes into btctax. It is provenance about \
                   the EVIDENCE, not a figure on the return — leave it blank and the packet's \
                   manifest names the row as transcribed without a date rather than hiding it.",
            kind: FieldKind::Date,
            live: |_| true,
            get: |ri, a| {
                ri.$vec
                    .get(a.0[0])
                    .map(|r| FieldValue::Date(r.transcribed_on))
            },
            set: |ri, a, v| {
                let FieldValue::Date(d) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.$vec
                    .get_mut(a.0[0])
                    .ok_or(SetError::NoSuchRow)?
                    .transcribed_on = d;
                Ok(())
            },
        }
    };
}

/// The `Repeating` section kind over a top-level `Vec` — `add` always has a container, `remove`
/// reports out-of-range (I-4).
macro_rules! doc_section_kind {
    ($vec:ident, $ty:ty) => {
        SectionKind::Repeating {
            len: |ri, _| ri.$vec.len(),
            add: |ri, _| {
                ri.$vec.push(<$ty>::default());
                Ok(())
            },
            remove: |ri, a| {
                if a.0[0] < ri.$vec.len() {
                    ri.$vec.remove(a.0[0]);
                    Ok(())
                } else {
                    Err(SetError::NoSuchRow)
                }
            },
        }
    };
}

/// The shared help for a payer TIN (R10.2 — the cross-year document identity).
const PAYER_TIN_HELP: &str =
    "The payer's TIN as printed on the form (\u{201c}PAYER'S TIN\u{201d}). It is the cross-year \
     identity of this payer: leaving it blank means NOT TRANSCRIBED, never \u{201c}the payer has \
     none\u{201d}.";

// ── 1099-INT ─────────────────────────────────────────────────────────────────────────────────────

const INT_1099_FIELDS: &[Field] = &[
    doc_text!(FieldId::Int1099Payer, int_1099, "PAYER'S name", "The payer as printed on the Form 1099-INT.", payer),
    doc_text!(FieldId::Int1099PayerTin, int_1099, "PAYER'S TIN", PAYER_TIN_HELP, payer_tin),
    doc_transcribed_on!(FieldId::Int1099TranscribedOn, int_1099),
    doc_money!(FieldId::Int1099Box1Interest, int_1099, "1 Interest income",
        "Box 1 \u{201c}Interest income\u{201d} \u{2014} Schedule B line 1, then Form 1040 line 2b.", box1_interest),
    doc_money!(FieldId::Int1099Box2EarlyWithdrawal, int_1099, "2 Early withdrawal penalty",
        "Box 2 \u{201c}Early withdrawal penalty\u{201d} \u{2014} Schedule 1 line 18, an adjustment to income.", box2_early_withdrawal_penalty),
    doc_money!(FieldId::Int1099Box3Treasury, int_1099, "3 Interest on U.S. Savings Bonds and Treasury obligations",
        "Box 3 \u{201c}Interest on U.S. Savings Bonds and Treasury obligations\u{201d} \u{2014} Form 1040 line 2b. It is NOT part of box 1: the two are added.", box3_treasury_interest),
    doc_money!(FieldId::Int1099Box4FedWithheld, int_1099, "4 Federal income tax withheld",
        "Box 4 \u{201c}Federal income tax withheld\u{201d} \u{2014} Form 1040 line 25b.", box4_fed_withheld),
    doc_money!(FieldId::Int1099Box6ForeignTax, int_1099, "6 Foreign tax paid",
        "Box 6 \u{201c}Foreign tax paid\u{201d} \u{2014} the \u{a7}904(j) no-Form-1116 foreign tax credit on Schedule 3 line 1. Above the $300/$600 ceiling it refuses.", box6_foreign_tax),
    doc_money!(FieldId::Int1099Box8TaxExempt, int_1099, "8 Tax-exempt interest",
        "Box 8 \u{201c}Tax-exempt interest\u{201d} \u{2014} Form 1040 line 2a. Reported, never taxed.", box8_tax_exempt_interest),
    doc_money!(FieldId::Int1099Box9PrivateActivity, int_1099, "9 Specified private activity bond interest",
        "Box 9 \u{201c}Specified private activity bond interest\u{201d} \u{2014} a Form 6251 AMT preference. btctax does not model it, so ANY amount here refuses rather than understate the AMT.", box9_private_activity_bond_amt),
    doc_money!(FieldId::Int1099Box10MarketDiscount, int_1099, "10 Market discount",
        "Box 10 \u{201c}Market discount\u{201d} \u{2014} Schedule B line 1 and the Form 1040 line 2b sum: \u{201c}Also include any accrued market discount that is includible in income\u{201d} (Schedule B instructions). Income, so leaving it out would understate your tax.", box10_market_discount),
    doc_money!(FieldId::Int1099Box11BondPremium, int_1099, "11 Bond premium",
        "Box 11 \u{201c}Bond premium\u{201d} \u{2014} \u{a7}171 amortizable bond premium REDUCES the interest you report, as a named Schedule B line-1 adjustment (Pub. 550). btctax computes no part of it, so any amount here refuses rather than report more interest than you owe tax on.", box11_bond_premium),
    doc_money!(FieldId::Int1099Box12BondPremiumTreasury, int_1099, "12 Bond premium on Treasury obligations",
        "Box 12 \u{201c}Bond premium on Treasury obligations\u{201d} \u{2014} as box 11: a \u{a7}171 reduction btctax does not compute, so any amount refuses.", box12_bond_premium_treasury),
    doc_money!(FieldId::Int1099Box13BondPremiumTaxExempt, int_1099, "13 Bond premium on tax-exempt bond",
        "Box 13 \u{201c}Bond premium on tax-exempt bond\u{201d} \u{2014} as box 11: a \u{a7}171 reduction btctax does not compute, so any amount refuses.", box13_bond_premium_tax_exempt),
];

pub(crate) const INT_1099S: Section = Section {
    id: SectionId::Int1099s,
    title: "Forms 1099-INT",
    kind: doc_section_kind!(int_1099, btctax_core::tax::return_inputs::Form1099Int),
    fields: INT_1099_FIELDS,
};

// ── 1099-DIV ─────────────────────────────────────────────────────────────────────────────────────

const DIV_1099_FIELDS: &[Field] = &[
    doc_text!(FieldId::Div1099Payer, div_1099, "PAYER'S name", "The payer as printed on the Form 1099-DIV.", payer),
    doc_text!(FieldId::Div1099PayerTin, div_1099, "PAYER'S TIN", PAYER_TIN_HELP, payer_tin),
    doc_transcribed_on!(FieldId::Div1099TranscribedOn, div_1099),
    doc_money!(FieldId::Div1099Box1aOrdinary, div_1099, "1a Total ordinary dividends",
        "Box 1a \u{201c}Total ordinary dividends\u{201d} \u{2014} Form 1040 line 3b. It ALREADY INCLUDES box 1b, so enter it as printed; btctax never adds the two.", box1a_ordinary),
    doc_money!(FieldId::Div1099Box1bQualified, div_1099, "1b Qualified dividends",
        "Box 1b \u{201c}Qualified dividends\u{201d} \u{2014} Form 1040 line 3a, the preferential-rate slice of box 1a. It may never exceed box 1a.", box1b_qualified),
    doc_money!(FieldId::Div1099Box2aCapGain, div_1099, "2a Total capital gain distr.",
        "Box 2a \u{201c}Total capital gain distr.\u{201d} \u{2014} Schedule D line 13, long-term.", box2a_capgain_distr),
    doc_money!(FieldId::Div1099Box2bUnrecap1250, div_1099, "2b Unrecap. Sec. 1250 gain",
        "Box 2b \u{201c}Unrecap. Sec. 1250 gain\u{201d} \u{2014} the 25% rate group needs the Schedule D unrecaptured-gain worksheet, which btctax does not build, so any amount refuses.", box2b_unrecap_1250),
    doc_money!(FieldId::Div1099Box2cSection1202, div_1099, "2c Section 1202 gain",
        "Box 2c \u{201c}Section 1202 gain\u{201d} \u{2014} the qualified small business stock exclusion, which btctax does not compute, so any amount refuses.", box2c_section_1202),
    doc_money!(FieldId::Div1099Box2dCollectibles, div_1099, "2d Collectibles (28%) gain",
        "Box 2d \u{201c}Collectibles (28%) gain\u{201d} \u{2014} the 28% rate group, which btctax does not compute, so any amount refuses.", box2d_collectibles_28),
    doc_money!(FieldId::Div1099Box4FedWithheld, div_1099, "4 Federal income tax withheld",
        "Box 4 \u{201c}Federal income tax withheld\u{201d} \u{2014} Form 1040 line 25b.", box4_fed_withheld),
    doc_money!(FieldId::Div1099Box5Section199a, div_1099, "5 Section 199A dividends",
        "Box 5 \u{201c}Section 199A dividends\u{201d} \u{2014} the REIT/PTP slice of box 1a that feeds the \u{a7}199A qualified business income deduction (Form 8995 line 6).", box5_section_199a),
    doc_money!(FieldId::Div1099Box7ForeignTax, div_1099, "7 Foreign tax paid",
        "Box 7 \u{201c}Foreign tax paid\u{201d} \u{2014} the \u{a7}904(j) no-Form-1116 foreign tax credit on Schedule 3 line 1.", box7_foreign_tax),
    doc_money!(FieldId::Div1099Box9CashLiquidation, div_1099, "9 Cash liquidation distributions",
        "Box 9 \u{201c}Cash liquidation distributions\u{201d} \u{2014} NOT a dividend: a liquidating distribution is treated as full payment in exchange for your stock, so it is a sale reported on Form 8949 and Schedule D in the year received, and the gain is the distribution less your basis. btctax holds no basis for that stock and builds no Form 8949 row for it, so any amount refuses rather than vanish.", box9_cash_liquidation),
    doc_money!(FieldId::Div1099Box10NoncashLiquidation, div_1099, "10 Noncash liquidation distributions",
        "Box 10 \u{201c}Noncash liquidation distributions\u{201d} \u{2014} as box 9, paid in kind rather than in cash: the same exchange treatment, the same missing basis, so any amount refuses.", box10_noncash_liquidation),
    doc_money!(FieldId::Div1099Box12ExemptInterest, div_1099, "12 Exempt-interest dividends",
        "Box 12 \u{201c}Exempt-interest dividends\u{201d} \u{2014} Form 1040 line 2a. Reported, never taxed.", box12_exempt_interest_dividends),
    doc_money!(FieldId::Div1099Box13PrivateActivity, div_1099, "13 Specified private activity bond interest dividends",
        "Box 13 \u{201c}Specified private activity bond interest dividends\u{201d} \u{2014} a Form 6251 AMT preference btctax does not model, so any amount refuses.", box13_private_activity_amt),
];

pub(crate) const DIV_1099S: Section = Section {
    id: SectionId::Div1099s,
    title: "Forms 1099-DIV",
    kind: doc_section_kind!(div_1099, btctax_core::tax::return_inputs::Form1099Div),
    fields: DIV_1099_FIELDS,
};

// ── 1099-B ───────────────────────────────────────────────────────────────────────────────────────

const B_1099_FIELDS: &[Field] = &[
    doc_text!(FieldId::B1099Payer, b_1099, "PAYER'S name (the broker)",
        "The broker, for your own records. Schedule D lines 1a/8a name no payer, so nothing on the printed return reads this \u{2014} it exists so three brokers are three tellable rows.", payer),
    doc_text!(FieldId::B1099PayerTin, b_1099, "PAYER'S TIN", PAYER_TIN_HELP, payer_tin),
    doc_transcribed_on!(FieldId::B1099TranscribedOn, b_1099),
    doc_money!(FieldId::B1099ShortTermProceeds, b_1099, "Short-term total: 1d Proceeds",
        "The SHORT-TERM total of box 1d \u{201c}Proceeds\u{201d} \u{2014} Schedule D line 1a, column (d).", short_term_proceeds),
    doc_money!(FieldId::B1099ShortTermBasis, b_1099, "Short-term total: 1e Cost or other basis",
        "The SHORT-TERM total of box 1e \u{201c}Cost or other basis\u{201d} \u{2014} Schedule D line 1a, column (e).", short_term_basis),
    doc_money!(FieldId::B1099LongTermProceeds, b_1099, "Long-term total: 1d Proceeds",
        "The LONG-TERM total of box 1d \u{201c}Proceeds\u{201d} \u{2014} Schedule D line 8a, column (d). Box 2 \u{201c}Short-term gain or loss\u{201d} on the form is what tells you which total a transaction joins.", long_term_proceeds),
    doc_money!(FieldId::B1099LongTermBasis, b_1099, "Long-term total: 1e Cost or other basis",
        "The LONG-TERM total of box 1e \u{201c}Cost or other basis\u{201d} \u{2014} Schedule D line 8a, column (e).", long_term_basis),
    doc_money!(FieldId::B1099Box13Bartering, b_1099, "13 Bartering",
        "Box 13 \u{201c}Bartering\u{201d} \u{2014} the fair market value of what a barter exchange arranged for you. It is income, and it reaches Schedule 1 line 8z \u{201c}Other income. List type and amount\u{201d}, or Schedule C if the bartering was in a trade or business. btctax fills line 8z from nothing and will not route income to a Schedule C you did not declare, so any amount refuses.", box13_bartering),
    Field {
        id: FieldId::B1099BasisReportedNoAdjustments,
        clear: Some(|ri, a| {
            ri.b_1099
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .basis_reported_and_no_adjustments = None;
            Ok(())
        }),
        label: "12 Check if basis reported to IRS \u{2014} AND no adjustments?",
        help: "Answer YES only if BOTH are true of every transaction on this Form 1099-B: box 12 \
               \u{201c}Basis reported to IRS\u{201d} is checked, AND you have no adjustments (no \
               wash sale in box 1g, no accrued market discount in box 1f, no disallowed loss in box \
               7, no noncovered security in box 5). Schedule D lines 1a and 8a accept totals only \
               on those two conditions; anything else belongs on Form 8949 one row at a time, which \
               btctax fills from its own crypto lot engine alone. Unanswered and NO both refuse.",
        kind: FieldKind::TriState,
        live: |_| true,
        get: |ri, a| {
            ri.b_1099
                .get(a.0[0])
                .map(|r| FieldValue::TriState(r.basis_reported_and_no_adjustments))
        },
        set: |ri, a, v| {
            let FieldValue::TriState(Some(b)) = v else {
                return Err(SetError::WrongKind);
            };
            ri.b_1099
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .basis_reported_and_no_adjustments = Some(b);
            Ok(())
        },
    },
];

pub(crate) const B_1099S: Section = Section {
    id: SectionId::B1099s,
    title: "Forms 1099-B (Schedule D summary totals)",
    kind: doc_section_kind!(b_1099, btctax_core::tax::return_inputs::Form1099B),
    fields: B_1099_FIELDS,
};

// ── 1099-G ───────────────────────────────────────────────────────────────────────────────────────

const G_1099_FIELDS: &[Field] = &[
    doc_text!(FieldId::G1099Payer, g_1099, "PAYER'S name", "The state or agency as printed on the Form 1099-G.", payer),
    doc_text!(FieldId::G1099PayerTin, g_1099, "PAYER'S TIN", PAYER_TIN_HELP, payer_tin),
    doc_transcribed_on!(FieldId::G1099TranscribedOn, g_1099),
    doc_money!(FieldId::G1099Box1Unemployment, g_1099, "1 Unemployment compensation",
        "Box 1 \u{201c}Unemployment compensation\u{201d} \u{2014} Schedule 1 line 7.", box1_unemployment),
    doc_money!(FieldId::G1099Box2StateRefund, g_1099, "2 State or local income tax refunds, credits, or offsets",
        "Box 2 \u{201c}State or local income tax refunds, credits, or offsets\u{201d} \u{2014} Schedule 1 line 1, but ONLY if you itemized on the return for the year you paid that tax (\u{a7}111(a)'s tax-benefit rule). btctax asks that question once for the return, because you owe the same answer whether or not a Form 1099-G arrived.", box2_state_refund),
    doc_money!(FieldId::G1099Box4FedWithheld, g_1099, "4 Federal income tax withheld",
        "Box 4 \u{201c}Federal income tax withheld\u{201d} \u{2014} Form 1040 line 25b.", box4_fed_withheld),
    doc_money!(FieldId::G1099Box5Rtaa, g_1099, "5 RTAA payments",
        "Box 5 \u{201c}RTAA payments\u{201d} \u{2014} Reemployment Trade Adjustment Assistance, which is income reaching Schedule 1 line 8z \u{201c}Other income. List type and amount\u{201d}. btctax fills line 8z from nothing, so any amount refuses rather than vanish.", box5_rtaa_payments),
    doc_money!(FieldId::G1099Box6TaxableGrants, g_1099, "6 Taxable grants",
        "Box 6 \u{201c}Taxable grants\u{201d} \u{2014} income reaching Schedule 1 line 8z, which btctax fills from nothing, so any amount refuses.", box6_taxable_grants),
    doc_money!(FieldId::G1099Box7Agriculture, g_1099, "7 Agriculture payments",
        "Box 7 \u{201c}Agriculture payments\u{201d} \u{2014} farm income, which reaches Schedule F. btctax does not produce a Schedule F, and its document census announces that exclusion only when you say you hold a farm document \u{2014} this figure arrives on a Form 1099-G instead, so any amount refuses rather than vanish unannounced.", box7_agriculture_payments),
    doc_money!(FieldId::G1099Box9MarketGain, g_1099, "9 Market gain",
        "Box 9 \u{201c}Market gain\u{201d} \u{2014} gain on the repayment of a Commodity Credit Corporation loan, which is farm income on Schedule F. As box 7, any amount refuses.", box9_market_gain),
    doc_money!(FieldId::G1099Box10FamilyLeave, g_1099, "10 Family leave benefits",
        "Box 10 \u{201c}Family leave benefits\u{201d} \u{2014} new on the December 2026 revision, for a state paid family and medical leave program (Rev. Rul. 2025-4). The benefits are income and reach Schedule 1 line 8z, which btctax fills from nothing, so any amount here refuses rather than vanish.", box10_family_leave_benefits),
];

pub(crate) const G_1099S: Section = Section {
    id: SectionId::G1099s,
    title: "Forms 1099-G",
    kind: doc_section_kind!(g_1099, btctax_core::tax::return_inputs::Form1099G),
    fields: G_1099_FIELDS,
};

// ── 1098-E ───────────────────────────────────────────────────────────────────────────────────────

const FORM_1098E_FIELDS: &[Field] = &[
    doc_text!(FieldId::Form1098eLender, form_1098e, "RECIPIENT'S/LENDER'S name", "The lender or servicer as printed on the Form 1098-E.", lender),
    doc_text!(FieldId::Form1098eLenderTin, form_1098e, "RECIPIENT'S/LENDER'S TIN", PAYER_TIN_HELP, lender_tin),
    doc_transcribed_on!(FieldId::Form1098eTranscribedOn, form_1098e),
    doc_money!(FieldId::Form1098eBox1Interest, form_1098e, "1 Student loan interest received by lender",
        "Box 1 \u{201c}Student loan interest received by lender\u{201d} \u{2014} Schedule 1 line 21, the \u{a7}221 deduction, after its $2,500 cap and its MAGI phase-out. btctax adds box 1 across every Form 1098-E on this return.", box1_interest),
];

pub(crate) const FORM_1098ES: Section = Section {
    id: SectionId::Form1098Es,
    title: "Forms 1098-E (student loan interest)",
    kind: doc_section_kind!(form_1098e, btctax_core::tax::return_inputs::Form1098E),
    fields: FORM_1098E_FIELDS,
};

// ── ★★★ R4 / T16 — Form 1099-SA and Form 5498-SA, the HSA information returns ───────────────────

/// The shared help for the three-way account checkbox both HSA information returns print.
///
/// ★★★ **Unanswered is not "HSA".** It is the box that decides WHICH FORM the figures belong on, so
/// leaving it blank refuses rather than defaulting — and the two MSA answers refuse naming Form
/// 8853, which the Form 8889 demands *"Before you begin"*.
const SA_ACCOUNT_TYPE_HELP: &str =
    "Which account does this document report? Tick it exactly as your form does. \u{201c}HSA\u{201d} is \
     the only one btctax can file: an Archer MSA or a Medicare Advantage MSA goes on FORM 8853, \
     which btctax does not build, so either answer refuses and names it. Leaving this blank refuses \
     too \u{2014} btctax will not assume HSA and file an MSA distribution on the wrong form.";

/// The Enum choices, which are the `SaAccountType` variant names.
const SA_ACCOUNT_TYPE_CHOICES: &[&str] = &["Hsa", "ArcherMsa", "MaMsa"];

fn sa_account_type_from(c: &str) -> Option<btctax_core::tax::return_inputs::SaAccountType> {
    use btctax_core::tax::return_inputs::SaAccountType as A;
    match c {
        "Hsa" => Some(A::Hsa),
        "ArcherMsa" => Some(A::ArcherMsa),
        "MaMsa" => Some(A::MaMsa),
        _ => None,
    }
}

const SA_1099_FIELDS: &[Field] = &[
    doc_text!(FieldId::Sa1099Payer, sa_1099, "TRUSTEE'S/PAYER'S name", "The HSA trustee or custodian as printed on the Form 1099-SA.", payer),
    doc_text!(FieldId::Sa1099PayerTin, sa_1099, "PAYER'S TIN", PAYER_TIN_HELP, payer_tin),
    doc_transcribed_on!(FieldId::Sa1099TranscribedOn, sa_1099),
    doc_money!(FieldId::Sa1099Box1GrossDistribution, sa_1099, "1 Gross distribution",
        "Box 1 \u{201c}Gross distribution\u{201d} \u{2014} Form 8889 line 14a, which adds box 1 across every Form 1099-SA on this return. It is the amount that came OUT of the account; how much of it is taxable is decided by line 15, the qualified medical expenses you paid with it.", box1_gross_distribution),
    doc_money!(FieldId::Sa1099Box2EarningsOnExcess, sa_1099, "2 Earnings on excess cont.",
        "Box 2 \u{201c}Earnings on excess cont.\u{201d} \u{2014} the earnings on excess contributions you withdrew. The form's own instruction to you is \u{201c}Include the earnings on the \u{2018}Other income\u{2019} line of your tax return\u{201d}, which is Schedule 1 line 8z \u{2014} a line btctax fills from nothing, so any amount here refuses rather than vanish.", box2_earnings_on_excess),
    doc_text!(FieldId::Sa1099Box3DistributionCode, sa_1099, "3 Distribution code",
        "Box 3 \u{201c}Distribution code\u{201d} \u{2014} the one-character code your trustee printed (1 normal, 2 excess contribution removed, 3 disability, 4 death, 5 prohibited transaction, 6 mistaken distribution). Transcribed for your records; Form 8889 line 17a asks YOU whether an exception applies, not the code.", box3_distribution_code),
    doc_money!(FieldId::Sa1099Box4Fmv, sa_1099, "4 FMV on date of death",
        "Box 4 \u{201c}FMV on date of death\u{201d} \u{2014} the account's fair market value when the owner died. If you inherited this account and were not the owner's spouse, that value is income to you on Schedule 1 line 8z, which btctax fills from nothing, so any amount here refuses.", box4_fmv_on_date_of_death),
    Field {
        id: FieldId::Sa1099Box5AccountType,
        clear: Some(|ri, a| {
            ri.sa_1099
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .box5_account_type = None;
            Ok(())
        }),
        label: "5 HSA / Archer MSA / MA MSA",
        help: SA_ACCOUNT_TYPE_HELP,
        kind: FieldKind::Enum(SA_ACCOUNT_TYPE_CHOICES),
        live: |_| true,
        get: |ri, a| {
            ri.sa_1099
                .get(a.0[0])
                .map(|r| match r.box5_account_type {
                    Some(v) => FieldValue::Choice(format!("{v:?}")),
                    None => FieldValue::Choice(String::new()),
                })
        },
        set: |ri, a, v| {
            let FieldValue::Choice(c) = v else {
                return Err(SetError::WrongKind);
            };
            let ty = sa_account_type_from(&c).ok_or(SetError::WrongKind)?;
            ri.sa_1099
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .box5_account_type = Some(ty);
            Ok(())
        },
    },
];

pub(crate) const SA_1099S: Section = Section {
    id: SectionId::Sa1099s,
    title: "Forms 1099-SA (HSA distributions)",
    kind: doc_section_kind!(sa_1099, btctax_core::tax::return_inputs::Form1099Sa),
    fields: SA_1099_FIELDS,
};

const SA_5498_FIELDS: &[Field] = &[
    doc_text!(FieldId::Sa5498Trustee, sa_5498, "TRUSTEE'S name", "The HSA trustee or custodian as printed on the Form 5498-SA.", trustee),
    doc_text!(FieldId::Sa5498TrusteeTin, sa_5498, "TRUSTEE'S TIN", PAYER_TIN_HELP, trustee_tin),
    doc_transcribed_on!(FieldId::Sa5498TranscribedOn, sa_5498),
    doc_money!(FieldId::Sa5498Box1ArcherContributions, sa_5498, "1 Employee\u{2019}s or self-employed person\u{2019}s Archer MSA contributions",
        "Box 1 \u{2014} the form prints it wrapped, \u{201c}1 Employee\u{2019}s or self-\u{201d} / \u{201c}employed person\u{2019}s Archer MSA contributions made in <year> and <year+1> for <year>\u{201d}. An Archer MSA belongs on FORM 8853, not Form 8889, so a Form 5498-SA reporting one refuses \u{2014} its box 6 is what says which account this is.", box1_archer_msa_contributions),
    doc_money!(FieldId::Sa5498Box2TotalContributions, sa_5498, "2 Total contributions made in the tax year",
        "Box 2 \u{2014} the form names its own year in the caption, so it reads \u{201c}Total contributions made in 2024\u{201d} on the 2024 edition and \u{201c}Total contributions made in 2025\u{201d} on the 2025 one. It is what your trustee received during the CALENDAR year, from you AND your employer together \u{2014} NOT Form 8889 line 2, which asks only for the contributions YOU made FOR the tax year. btctax transcribes it so you can check line 2 against it, and never adds it to anything.", box2_total_contributions),
    doc_money!(FieldId::Sa5498Box3NextYearForThisYear, sa_5498, "3 Contributions made after the year end, for the tax year",
        "Box 3 \u{2014} again the form names its own years: \u{201c}Total HSA or Archer MSA contributions made in 2025 for 2024\u{201d} on the 2024 edition, \u{201c}Total HSA or Archer MSA contributions made in 2026 for 2025\u{201d} on the 2025 one. Contributions your trustee received after the year ended but designated FOR it, up to the April filing deadline. Form 8889 line 2 includes such contributions if YOU made them; this box is what lets you check that.", box3_contributions_next_year_for_this_year),
    doc_money!(FieldId::Sa5498Box4Rollover, sa_5498, "4 Rollover contributions",
        "Box 4 \u{201c}Rollover contributions\u{201d} \u{2014} money moved from another HSA or Archer MSA. Form 8889 line 2's instruction says explicitly not to include rollovers, so this figure reaches no line.", box4_rollover_contributions),
    doc_money!(FieldId::Sa5498Box5Fmv, sa_5498, "5 Fair market value of HSA, Archer MSA, or MA MSA",
        "Box 5 \u{201c}Fair market value of HSA, Archer MSA, or MA MSA\u{201d} \u{2014} what the account was worth at the end of the year. No line of Form 8889 or the Form 1040 chain reads it; it is transcribed so your record of the account is complete.", box5_fair_market_value),
    Field {
        id: FieldId::Sa5498Box6AccountType,
        clear: Some(|ri, a| {
            ri.sa_5498
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .box6_account_type = None;
            Ok(())
        }),
        label: "6 HSA / Archer MSA / MA MSA",
        help: SA_ACCOUNT_TYPE_HELP,
        kind: FieldKind::Enum(SA_ACCOUNT_TYPE_CHOICES),
        live: |_| true,
        get: |ri, a| {
            ri.sa_5498
                .get(a.0[0])
                .map(|r| match r.box6_account_type {
                    Some(v) => FieldValue::Choice(format!("{v:?}")),
                    None => FieldValue::Choice(String::new()),
                })
        },
        set: |ri, a, v| {
            let FieldValue::Choice(c) = v else {
                return Err(SetError::WrongKind);
            };
            let ty = sa_account_type_from(&c).ok_or(SetError::WrongKind)?;
            ri.sa_5498
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .box6_account_type = Some(ty);
            Ok(())
        },
    },
];

pub(crate) const SA_5498S: Section = Section {
    id: SectionId::Sa5498s,
    title: "Forms 5498-SA (HSA contribution information)",
    kind: doc_section_kind!(sa_5498, btctax_core::tax::return_inputs::Form5498Sa),
    fields: SA_5498_FIELDS,
};

// ── ★★★ T16 — Form 8889's own money leaves ──────────────────────────────────────────────────────
//
// LIVE only when the §223 trigger declaration is affirmed, so a filer with no HSA activity is never
// shown a figure they have no reason to enter.

/// The one liveness predicate for the Form 8889 money section — the same condition the seven
/// declarations use, read from the same leaf.
fn form_8889_live(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> bool {
    ri.sch1.hsa_activity == Some(true)
}

/// A singleton money leaf over `ri.hsa.$field`, live iff Form 8889 files.
macro_rules! hsa_money {
    ($id:expr, $label:literal, $help:expr, $field:ident) => {
        Field {
            id: $id,
            clear: None,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: form_8889_live,
            get: |ri, _| form_8889_live(ri).then(|| FieldValue::Money(ri.hsa.$field)),
            set: |ri, _, v| {
                if !form_8889_live(ri) {
                    return Err(SetError::NoSuchRow);
                }
                let FieldValue::Money(m) = v else {
                    return Err(SetError::WrongKind);
                };
                ri.hsa.$field = m;
                Ok(())
            },
        }
    };
}

const FORM_8889_FIELDS: &[Field] = &[
    hsa_money!(FieldId::HsaLine2Contributions, "2 HSA contributions you made for this year",
        "Line 2 \u{2014} \u{201c}HSA contributions you made for <year> (or those made on your behalf), including those made by the unextended due date of your tax return that were for <year>. Do NOT include employer contributions, contributions through a cafeteria plan, or rollovers.\u{201d} Your employer's share comes from Form W-2 box 12 code W and is never re-asked; a cafeteria-plan payroll deduction counts as your EMPLOYER'S contribution, not yours.", line2_contributions_you_made),
    hsa_money!(FieldId::HsaEmployerPriorYear, "Employer Contribution Worksheet line 2 \u{2014} contributions made THIS year for LAST year",
        "The Employer Contribution Worksheet in the Instructions for Form 8889: \u{201c}Enter employer contributions made in <year> for tax year <year-1>.\u{201d} A Form W-2 reports by CALENDAR year and line 9 wants the TAX year, so this is subtracted from your box 12 code W total. Leave it at 0 unless your W-2's code W includes last year's contribution.", employer_contributions_prior_year),
    hsa_money!(FieldId::HsaEmployerNextYear, "Employer Contribution Worksheet line 4 \u{2014} contributions made NEXT year for this year",
        "The Employer Contribution Worksheet: \u{201c}Enter employer contributions made in <year+1> for tax year <year>.\u{201d} Added to your box 12 code W total for the same calendar-versus-tax-year reason. Leave it at 0 unless your employer contributed after the year ended and designated it for this year.", employer_contributions_next_year),
    hsa_money!(FieldId::HsaLine10FundingDistribution, "10 Qualified HSA funding distributions",
        "Line 10 \u{2014} \u{201c}Qualified HSA funding distributions.\u{201d} A once-in-a-lifetime direct trustee-to-trustee transfer from your traditional or Roth IRA into your HSA. It is not distributed FROM the HSA, so no Form 1099-SA reports it \u{2014} your own records are the source. It is not deductible and it REDUCES what you may contribute.", line10_qualified_funding_distribution),
    hsa_money!(FieldId::HsaLine14bRollovers, "14b Rollovers and excess contributions withdrawn by the due date",
        "Line 14b \u{2014} \u{201c}Distributions included on line 14a that you rolled over to another HSA. Also include any excess contributions (and the earnings on those excess contributions) included on line 14a that were withdrawn by the due date of your return.\u{201d} The Form 1099-SA does not distinguish a rollover, so this comes from your records.", line14b_rollovers_and_withdrawn_excess),
    hsa_money!(FieldId::HsaLine15MedicalExpenses, "15 Qualified medical expenses paid using HSA distributions",
        "Line 15 \u{2014} \u{201c}Qualified medical expenses paid using HSA distributions.\u{201d} THE figure that decides how much of your distribution is taxable, and no document carries it: the Form 1099-SA's own instruction says \u{201c}The payer isn't required to compute the taxable amount of any distribution.\u{201d} Only expenses not reimbursed by insurance, incurred after the HSA was established, for you, your spouse and your dependents. You cannot also deduct these on Schedule A.", line15_qualified_medical_expenses),
    hsa_money!(FieldId::HsaLine16Excepted, "17a/17b The part of line 16 that meets an exception to the additional 20% tax",
        "Line 17b \u{2014} \u{201c}Enter on line 17b only 20% (0.20) of any amount included on line 16 that does not meet any of the exceptions.\u{201d} So enter here the part of your TAXABLE distribution that DOES meet one: distributions made after the account beneficiary dies, becomes disabled, or turns age 65. Any amount here also checks the line 17a box, which is the form's own sentence about it. Leave it at 0 if none applies.", line16_amount_meeting_an_exception),
];

pub(crate) const FORM_8889: Section = Section {
    id: SectionId::Form8889,
    title: "Form 8889 (health savings account)",
    kind: SectionKind::Singleton,
    fields: FORM_8889_FIELDS,
};

// ── ★★★ R5 — the filer's-records rows for Schedule B lines 1 and 5 ──────────────────────────────
//
// LIVE only when the filer has said such income exists, so nobody is shown a section for income
// they do not have; non-empty is then REQUIRED (`FilerRecordsDeclaredNotTranscribed`).

/// **The R3 door itself** — the answer that AUTHORISES a new row, in one place. `add` is the only
/// caller: creating a row the filer never opened the door for would be testimony they never gave.
fn schedule_b_door_open(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> bool {
    btctax_core::tax::questions::question_is_live(
        btctax_core::tax::questions::QuestionId::InterestOrDividendsWithout1099,
        ri,
    ) && ri.interest_or_dividends_without_1099 == Some(true)
}

/// The section's liveness — the door open **OR** rows already present.
///
/// ★★★ Seam review I-2. Liveness used to be the door alone, which made a closed door with rows
/// behind it unreachable: all five `Field`s went dead, `add` refused, and the orphan rows kept
/// printing on Schedule B and in the 1040 line 2b/3b sums — a figure on the filer's own return that
/// no surface could show, edit or withdraw. The return now refuses in that state
/// ([`btctax_core::tax::return_refuse::RefuseReason::FilerRecordsContradicted`]), and the refusal
/// has to have an exit, so the rows stay VISIBLE and removable while they exist.
fn schedule_b_records_live(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> bool {
    schedule_b_door_open(ri) || !ri.schedule_b_filer_records.is_empty()
}

const SB_RECORD_FIELDS: &[Field] = &[
    Field {
        id: FieldId::SbRecordPayerName,
        clear: None,
        label: "Name of payer",
        help: "Schedule B line 1 / line 5, the \u{201c}List name of payer\u{201d} column \u{2014} \
               who paid you. For a seller-financed mortgage this is the BUYER.",
        kind: FieldKind::Text,
        live: schedule_b_records_live,
        get: |ri, a| {
            ri.schedule_b_filer_records
                .get(a.0[0])
                .map(|r| FieldValue::Text(r.payer_name.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(t) = v else { return Err(SetError::WrongKind) };
            ri.schedule_b_filer_records
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .payer_name = t;
            Ok(())
        },
    },
    Field {
        id: FieldId::SbRecordPayerSsn,
        clear: None,
        label: "Buyer's SSN (seller-financed mortgage only)",
        help: "Schedule B: \u{201c}If you sold your home or other property and the buyer used the \
               property as a personal residence, list first any interest the buyer paid you on a \
               mortgage \u{2026} and show that buyer's social security number (SSN) and address.\u{201d} \
               Leave it blank for every other kind of row.",
        kind: FieldKind::Secret,
        live: schedule_b_records_live,
        get: |ri, a| {
            ri.schedule_b_filer_records
                .get(a.0[0])
                .map(|r| FieldValue::Secret(mask_ssn(&r.payer_ssn)))
        },
        set: |ri, a, v| {
            let FieldValue::SecretEntry(t) = v else { return Err(SetError::WrongKind) };
            ri.schedule_b_filer_records
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .payer_ssn = t;
            Ok(())
        },
    },
    Field {
        id: FieldId::SbRecordPayerAddress,
        clear: None,
        label: "Buyer's address (seller-financed mortgage only)",
        help: "The buyer's address, which Schedule B asks for beside their SSN on a seller-financed \
               mortgage. Leave it blank for every other kind of row.",
        kind: FieldKind::Text,
        live: schedule_b_records_live,
        get: |ri, a| {
            ri.schedule_b_filer_records
                .get(a.0[0])
                .map(|r| FieldValue::Text(r.payer_address.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(t) = v else { return Err(SetError::WrongKind) };
            ri.schedule_b_filer_records
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .payer_address = t;
            Ok(())
        },
    },
    Field {
        id: FieldId::SbRecordAmount,
        clear: None,
        label: "Amount",
        help: "The amount, from your own records. Schedule B line 1 says to report ALL of your \
               taxable interest, whether or not a payer sent you a Form 1099-INT.",
        kind: FieldKind::Money,
        live: schedule_b_records_live,
        get: |ri, a| {
            ri.schedule_b_filer_records
                .get(a.0[0])
                .map(|r| FieldValue::Money(r.amount))
        },
        set: |ri, a, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.schedule_b_filer_records
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .amount = m;
            Ok(())
        },
    },
    Field {
        id: FieldId::SbRecordKind,
        clear: None,
        label: "Interest or dividend?",
        help: "Which Schedule B list this row joins: Interest goes on line 1 (and carries to Form \
               1040 line 2b); Dividend goes on line 5 (and carries to Form 1040 line 3b).",
        kind: FieldKind::Enum(&["Interest", "Dividend"]),
        live: schedule_b_records_live,
        get: |ri, a| {
            ri.schedule_b_filer_records
                .get(a.0[0])
                .map(|r| FieldValue::Choice(format!("{:?}", r.kind)))
        },
        set: |ri, a, v| {
            use btctax_core::tax::return_inputs::ScheduleBRecordKind as K;
            let FieldValue::Choice(c) = v else { return Err(SetError::WrongKind) };
            let kind = match c.as_str() {
                "Interest" => K::Interest,
                "Dividend" => K::Dividend,
                _ => return Err(SetError::WrongKind),
            };
            ri.schedule_b_filer_records
                .get_mut(a.0[0])
                .ok_or(SetError::NoSuchRow)?
                .kind = kind;
            Ok(())
        },
    },
];

pub(crate) const SCHEDULE_B_FILER_RECORDS: Section = Section {
    id: SectionId::ScheduleBFilerRecords,
    title: "Interest and dividends from your own records",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.schedule_b_filer_records.len(),
        // ★ `add` stays on the DOOR — not on the section's liveness, which is now also true while
        //   orphan rows exist (I-2). Adding a row to a return that has not opened the door would be
        //   testimony the filer never gave; removing one that is already there is the exit.
        add: |ri, _| {
            if !schedule_b_door_open(ri) {
                return Err(SetError::NoSuchRow);
            }
            ri.schedule_b_filer_records
                .push(btctax_core::tax::return_inputs::ScheduleBRecord::default());
            Ok(())
        },
        remove: |ri, a| {
            if a.0[0] < ri.schedule_b_filer_records.len() {
                ri.schedule_b_filer_records.remove(a.0[0]);
                Ok(())
            } else {
                Err(SetError::NoSuchRow)
            }
        },
    },
    fields: SB_RECORD_FIELDS,
};

#[cfg(test)]
mod filer_records_tests {
    use super::*;
    use crate::seam::RowAddr;
    use btctax_core::tax::document_census::DocumentRow;
    use btctax_core::tax::questions::{question_is_live, QuestionId};
    use btctax_core::tax::return_inputs::{
        Form1099Div, Form1099Int, ReturnInputs, ScheduleBRecord,
    };

    /// ★★★ **SEAM REVIEW I-2's OTHER HALF — the refusal's EXIT.**
    ///
    /// `FilerRecordsContradicted` refuses a return whose filer's-records rows no answer authorises.
    /// A refusal with no exit is a brick, and the exit is the section: while rows exist they must
    /// stay VISIBLE so the filer can remove them — even with the door shut. `add` is the one thing
    /// that stays on the door, because creating a row nobody opened the door for would be testimony
    /// the filer never gave.
    #[test]
    fn orphan_filer_records_stay_visible_while_add_stays_on_the_door() {
        // The door shut the hard way: the filer now says they hold BOTH documents.
        let mut ri = ReturnInputs::default();
        for row in [DocumentRow::Int1099, DocumentRow::Div1099] {
            ri.documents.set(row, Some(true));
        }
        ri.int_1099.push(Form1099Int::default());
        ri.div_1099.push(Form1099Div::default());
        ri.interest_or_dividends_without_1099 = Some(true);
        assert!(
            !question_is_live(QuestionId::InterestOrDividendsWithout1099, &ri),
            "the probe's premise: holding both documents closes the door"
        );

        // With no rows the section is dead — nobody is shown a section for income they do not have.
        assert!(
            SB_RECORD_FIELDS.iter().all(|f| !(f.live)(&ri)),
            "a closed door with NO rows must leave the section invisible"
        );
        assert!(
            matches!(SCHEDULE_B_FILER_RECORDS.kind, SectionKind::Repeating { add, .. }
                     if add(&mut ri.clone(), &RowAddr(vec![])).is_err()),
            "…and `add` must refuse there"
        );

        // One orphan row, and the section comes back — that is the exit the refusal needs.
        ri.schedule_b_filer_records.push(ScheduleBRecord::default());
        assert!(
            SB_RECORD_FIELDS.iter().all(|f| (f.live)(&ri)),
            "★ THE KILL: with the door closed and a row on the return the section must STAY \
             VISIBLE, or the filer cannot remove the figure `FilerRecordsContradicted` is about"
        );
        let SectionKind::Repeating { add, remove, .. } = SCHEDULE_B_FILER_RECORDS.kind else {
            panic!("the filer's-records section is repeating");
        };
        assert!(
            add(&mut ri.clone(), &RowAddr(vec![])).is_err(),
            "★ THE KILL: `add` stays on the DOOR — a visible section is not authorisation to \
             create testimony the filer never gave"
        );
        assert!(
            remove(&mut ri, &RowAddr(vec![0])).is_ok() && ri.schedule_b_filer_records.is_empty(),
            "…and `remove` is the exit, so the return becomes fileable again"
        );
    }
}
