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
    Field, FieldId, FieldKind, FieldValue, Section, SectionId, SectionKind, SecretView, SetError,
};
use btctax_core::conventions::Usd;
use btctax_core::tax::questions::{FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
use btctax_core::tax::return_inputs::{
    Box12Entry, CharitableClass, CharitableGift, Dependent, ItemizeElection, Person,
    ScheduleAInputs, W2,
};
use btctax_core::tax::types::FilingStatus;

// ── The Secret masker — the ONE constructor of `SecretView::Set` in this crate ─────────────────────────────
// A caller cannot store raw digits through it: it emits only the `***-**-NNNN` presence shape (spec §4/§5.5),
// showing at most the last four characters. Empty input → `Empty`. Canonical validation of the raw entry is
// Task 8's `parse`, upstream of the `SecretEntry` this masker never sees.
fn mask_secret(raw: &str) -> SecretView {
    if raw.is_empty() {
        return SecretView::Empty;
    }
    let n = raw.chars().count();
    let last4: String = raw.chars().skip(n.saturating_sub(4)).collect();
    SecretView::Set { masked: format!("***-**-{last4}") }
}

// ── Leaf generators for the repetitive money families (one struct-field ident is all that varies) ──────────

/// A repeating-W-2-row `Money` leaf over `ri.w2s[addr.0[0]].$field`.
macro_rules! w2_money {
    ($id:expr, $label:literal, $help:literal, $field:ident) => {
        Field {
            id: $id,
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Money(w.$field)),
            set: |ri, a, v| {
                let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
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
            label: $label,
            help: $help,
            kind: FieldKind::Money,
            live: |_| true,
            get: |ri, _| ri.schedule_a.as_ref().map(|a| FieldValue::Money(a.$field)),
            set: |ri, _, v| {
                let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
                ri.schedule_a.as_mut().ok_or(SetError::NoSuchRow)?.$field = m;
                Ok(())
            },
        }
    };
}

// ── 1. ReturnOptions (Singleton) — the filing-status/itemize-election header ────────────────────────────────

const RETURN_OPTIONS_FIELDS: &[Field] = &[
    Field {
        id: FieldId::FilingStatus,
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
        label: "Itemize election",
        help: "Auto = take the larger of standard vs Schedule A; ForceItemize = §63(e) elect to itemize \
               even if smaller.",
        kind: FieldKind::Enum(&["Auto", "ForceItemize"]),
        live: |_| true,
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
        label: "SSN",
        help: "The taxpayer's Social Security number. Stored as entered; shown masked (`***-**-NNNN`).",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Secret(mask_secret(&ri.header.taxpayer.ssn))),
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.taxpayer.ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::TpOccupation,
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
        label: "Identity Protection PIN",
        help: "The IRS-issued 6-digit IP PIN, if you have one. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| {
            Some(FieldValue::Secret(ri.header.ip_pin.as_deref().map_or(SecretView::Empty, mask_secret)))
        },
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.ip_pin = Some(s);
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
        label: "Spouse SSN",
        help: "The spouse's Social Security number. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, _| ri.header.spouse.as_ref().map(|s| FieldValue::Secret(mask_secret(&s.ssn))),
        set: |ri, _, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.spouse.as_mut().ok_or(SetError::NoSuchRow)?.ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::SpOccupation,
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
        label: "Street address",
        help: "Home address — street (1040 header).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_street.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.address_street = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrCity,
        label: "City",
        help: "Home address — city or town.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_city.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.address_city = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrState,
        label: "State",
        help: "Home address — state (two-letter USPS code).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_state.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.address_state = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::AddrZip,
        label: "ZIP code",
        help: "Home address — ZIP code.",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Text(ri.header.address_zip.clone())),
        set: |ri, _, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
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

// ── 5. Dependents (Repeating) — `header.dependents: Vec<Dependent>`, indexed by `addr.0[0]` ─────────────────

const DEPENDENT_FIELDS: &[Field] = &[
    Field {
        id: FieldId::DepName,
        label: "Dependent name",
        help: "The dependent's full name (1040 dependents grid).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| ri.header.dependents.get(a.0[0]).map(|d| FieldValue::Text(d.name.clone())),
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.dependents.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.name = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepSsn,
        label: "Dependent SSN",
        help: "The dependent's Social Security number. Stored as entered; shown masked.",
        kind: FieldKind::Secret,
        live: |_| true,
        get: |ri, a| {
            ri.header.dependents.get(a.0[0]).map(|d| FieldValue::Secret(mask_secret(&d.ssn)))
        },
        set: |ri, a, v| {
            let FieldValue::SecretEntry(s) = v else { return Err(SetError::WrongKind) };
            ri.header.dependents.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.ssn = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepRelationship,
        label: "Relationship",
        help: "The dependent's relationship to you (e.g. son, daughter, parent).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| {
            ri.header.dependents.get(a.0[0]).map(|d| FieldValue::Text(d.relationship.clone()))
        },
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.header.dependents.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.relationship = s;
            Ok(())
        },
    },
    Field {
        id: FieldId::DepDob,
        label: "Date of birth",
        help: "The dependent's date of birth.",
        kind: FieldKind::Date,
        live: |_| true,
        get: |ri, a| ri.header.dependents.get(a.0[0]).map(|d| FieldValue::Date(d.date_of_birth)),
        set: |ri, a, v| {
            let FieldValue::Date(d) = v else { return Err(SetError::WrongKind) };
            ri.header.dependents.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.date_of_birth = d;
            Ok(())
        },
    },
];

pub(crate) const DEPENDENTS: Section = Section {
    id: SectionId::Dependents,
    title: "Dependents",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.header.dependents.len(),
        add: |ri, _| ri.header.dependents.push(Dependent::default()),
        remove: |ri, a| {
            if a.0[0] < ri.header.dependents.len() {
                ri.header.dependents.remove(a.0[0]);
            }
        },
    },
    fields: DEPENDENT_FIELDS,
};

// ── 6. W2s (Repeating) — `ri.w2s: Vec<W2>`, indexed by `addr.0[0]` ──────────────────────────────────────────

const W2_FIELDS: &[Field] = &[
    Field {
        id: FieldId::W2Owner,
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
        label: "Employer",
        help: "The employer's name (W-2 box c).",
        kind: FieldKind::Text,
        live: |_| true,
        get: |ri, a| ri.w2s.get(a.0[0]).map(|w| FieldValue::Text(w.employer.clone())),
        set: |ri, a, v| {
            let FieldValue::Text(s) = v else { return Err(SetError::WrongKind) };
            ri.w2s.get_mut(a.0[0]).ok_or(SetError::NoSuchRow)?.employer = s;
            Ok(())
        },
    },
    w2_money!(FieldId::Box1Wages, "Box 1 — wages", "W-2 box 1 (wages, tips, other comp.)", box1_wages),
    w2_money!(FieldId::Box2FedWh, "Box 2 — federal tax withheld", "W-2 box 2 → 1040 25a", box2_fed_withheld),
    w2_money!(FieldId::Box3SsWages, "Box 3 — Social Security wages", "W-2 box 3 (per-earner SS cap)", box3_ss_wages),
    w2_money!(FieldId::Box4SsWh, "Box 4 — Social Security tax withheld", "W-2 box 4 → excess-SS credit (§4.9)", box4_ss_withheld),
    w2_money!(FieldId::Box5MedWages, "Box 5 — Medicare wages", "W-2 box 5 → Form 8959 Part I", box5_medicare_wages),
    w2_money!(FieldId::Box6MedWh, "Box 6 — Medicare tax withheld", "W-2 box 6 → Form 8959 Part V", box6_medicare_withheld),
    w2_money!(FieldId::Box7SsTips, "Box 7 — Social Security tips", "W-2 box 7", box7_ss_tips),
    w2_money!(FieldId::Box17StateWh, "Box 17 — state income tax", "W-2 box 17 → Sch A 5a (income-tax path)", box17_state_tax_withheld),
    w2_money!(FieldId::Box19LocalTax, "Box 19 — local income tax", "W-2 box 19 → Sch A 5a", box19_local_tax),
    w2_money!(FieldId::Box8AllocTips, "Box 8 — allocated tips", "W-2 box 8 (refuse-guard if > 0, §4.10)", box8_allocated_tips),
    w2_money!(FieldId::Box10DepCare, "Box 10 — dependent-care benefits", "W-2 box 10 (refuse-guard if > 0, §4.10)", box10_dependent_care),
];

pub(crate) const W2S: Section = Section {
    id: SectionId::W2s,
    title: "W-2s",
    kind: SectionKind::Repeating {
        len: |ri, _| ri.w2s.len(),
        add: |ri, _| ri.w2s.push(W2::default()),
        remove: |ri, a| {
            if a.0[0] < ri.w2s.len() {
                ri.w2s.remove(a.0[0]);
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
            if let Some(w) = ri.w2s.get_mut(a.0[0]) {
                // `Box12Entry` has no `Default` (a blank code + zero dollars is the empty new row).
                w.box12.push(Box12Entry { code: String::new(), amount: Usd::ZERO });
            }
        },
        remove: |ri, a| {
            if let Some(w) = ri.w2s.get_mut(a.0[0]) {
                if a.0[1] < w.box12.len() {
                    w.box12.remove(a.0[1]);
                }
            }
        },
    },
    fields: W2_BOX12_FIELDS,
};

// ── 8. ScheduleA (OptionalSingleton) — 7 money leaves + the 2 registry-driven tri-states ────────────────────

const SCHEDULE_A_FIELDS: &[Field] = &[
    scha_money!(FieldId::SaMedical, "Medical expenses", "Sch A line 1 — medical/dental (§213 7.5% floor).", medical),
    scha_money!(FieldId::SaSaltRealEstate, "Real-estate taxes", "Sch A line 5b — real-estate taxes.", salt_real_estate),
    scha_money!(FieldId::SaSaltPersonalProp, "Personal-property taxes", "Sch A line 5c — personal-property taxes.", salt_personal_property),
    scha_money!(FieldId::SaSaltStateEst, "State estimated payments", "State/local income-tax estimated payments (income-tax path, line 5a).", salt_state_estimated_payments),
    scha_money!(FieldId::SaSaltPriorYear, "Prior-year balance paid", "State/local income tax — prior-year balance paid this year (income-tax path).", salt_prior_year_balance_paid),
    scha_money!(FieldId::SaSaltSalesTaxAmt, "General sales-tax amount", "Sch A line 5a sales-tax amount — used iff the §164(b)(5) sales-tax election is Yes.", salt_sales_tax_amount),
    scha_money!(FieldId::SaMortgage1098, "Home-mortgage interest (1098)", "Sch A line 8a — mortgage interest reported on Form 1098.", mortgage_interest_1098),
    // ★ Registry-driven — DELEGATES to `SKIPPABLE_QUESTIONS::SalesTaxElection` (index 2). live = schedule_a.is_some().
    skippable_tristate!(2, FieldId::SaSaltUseSalesTax),
    // ★ Registry-driven — DELEGATES to `FORM_QUESTIONS::MortgageAllUsedToBuyBuildImprove` (index 7).
    decl_tristate!(7, FieldId::SaMortgageAllUsed),
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
            if let Some(sa) = ri.schedule_a.as_mut() {
                // `CharitableGift`/`CharitableClass` have no `Default`; a cash gift to a 50%-org is the
                // most common class and a safe starting point (the filer then picks the real class).
                sa.charitable.push(CharitableGift { class: CharitableClass::Cash60, amount: Usd::ZERO });
            }
        },
        remove: |ri, a| {
            if let Some(sa) = ri.schedule_a.as_mut() {
                if a.0[0] < sa.charitable.len() {
                    sa.charitable.remove(a.0[0]);
                }
            }
        },
    },
    fields: CHARITABLE_FIELDS,
};

// ── 10. Payments (Singleton) — `ri.payments` ────────────────────────────────────────────────────────────────

const PAYMENTS_FIELDS: &[Field] = &[
    Field {
        id: FieldId::PayEstimated,
        label: "Estimated tax payments",
        help: "§6654 estimated-tax payments made for the year → 1040 line 26.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.estimated_tax_payments)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.payments.estimated_tax_payments = m;
            Ok(())
        },
    },
    Field {
        id: FieldId::PayExtension,
        label: "Extension payment",
        help: "Amount paid with a Form 4868 extension request → Sch 3 line 10.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.extension_payment)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.payments.extension_payment = m;
            Ok(())
        },
    },
    Field {
        id: FieldId::PayOtherWh,
        label: "Other withholding",
        help: "Other federal income tax withheld (e.g. Form 1099 backup withholding) → 1040 line 25c.",
        kind: FieldKind::Money,
        live: |_| true,
        get: |ri, _| Some(FieldValue::Money(ri.payments.other_withholding)),
        set: |ri, _, v| {
            let FieldValue::Money(m) = v else { return Err(SetError::WrongKind) };
            ri.payments.other_withholding = m;
            Ok(())
        },
    },
];

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
        FieldId, FieldKind, FieldValue, RowAddr, SectionId, SectionKind, SecretView, SetError,
    };
    use btctax_core::tax::return_inputs::ItemizeElection;
    use btctax_core::tax::types::FilingStatus;
    use rust_decimal_macros::dec;

    #[test]
    fn w2_repeating_with_nested_box12_reads_and_writes() {
        let mut ri = fresh_single();
        let w2s = section(SectionId::W2s);
        let SectionKind::Repeating { add, len, .. } = w2s.kind else { panic!() };
        (add)(&mut ri, &RowAddr::default());
        assert_eq!((len)(&ri, &RowAddr::default()), 1);
        let box1 = w2s.fields.iter().find(|f| f.id == FieldId::Box1Wages).unwrap();
        (box1.set)(&mut ri, &RowAddr(vec![0]), FieldValue::Money(dec!(50000))).unwrap();
        assert_eq!(ri.w2s[0].box1_wages, dec!(50000));
        // nested box12
        let b12 = section(SectionId::W2Box12);
        let SectionKind::Repeating { add: add12, .. } = b12.kind else { panic!() };
        (add12)(&mut ri, &RowAddr(vec![0])); // parent = w2 index 0
        assert_eq!(ri.w2s[0].box12.len(), 1);
        let amt = b12.fields.iter().find(|f| f.id == FieldId::Box12Amount).unwrap();
        (amt.set)(&mut ri, &RowAddr(vec![0, 0]), FieldValue::Money(dec!(23000))).unwrap();
        assert_eq!(ri.w2s[0].box12[0].amount, dec!(23000));
    }

    #[test]
    fn schedule_a_optional_singleton_create_delete_resets_itemize_election() {
        let mut ri = fresh_single();
        ri.itemize_election = ItemizeElection::ForceItemize;
        let sa = section(SectionId::ScheduleA);
        let SectionKind::OptionalSingleton { create, delete, present } = sa.kind else { panic!() };
        (create)(&mut ri);
        assert!((present)(&ri));
        (delete)(&mut ri);
        assert!(!(present)(&ri));
        assert_eq!(ri.itemize_election, ItemizeElection::Auto, "I-10: delete resets ForceItemize");
    }

    #[test]
    fn singleton_and_optional_singleton_get_set_spotcheck() {
        let mut ri = fresh_single();
        // ReturnOptions singleton: FilingStatus enum roundtrip + wrong-choice rejection.
        let ro = section(SectionId::ReturnOptions);
        let fs = ro.fields.iter().find(|f| f.id == FieldId::FilingStatus).unwrap();
        assert_eq!((fs.get)(&ri, &RowAddr::default()), Some(FieldValue::Choice("Single".into())));
        (fs.set)(&mut ri, &RowAddr::default(), FieldValue::Choice("Mfj".into())).unwrap();
        assert_eq!(ri.filing_status, FilingStatus::Mfj);
        assert_eq!(
            (fs.set)(&mut ri, &RowAddr::default(), FieldValue::Choice("Nope".into())),
            Err(SetError::WrongKind)
        );

        // Payments singleton money.
        let pay = section(SectionId::Payments);
        let est = pay.fields.iter().find(|f| f.id == FieldId::PayEstimated).unwrap();
        (est.set)(&mut ri, &RowAddr::default(), FieldValue::Money(dec!(1200))).unwrap();
        assert_eq!(ri.payments.estimated_tax_payments, dec!(1200));

        // Spouse optional-singleton: get None + set NoSuchRow until created.
        let sp = section(SectionId::Spouse);
        let SectionKind::OptionalSingleton { create, present, .. } = sp.kind else { panic!() };
        let sp_first = sp.fields.iter().find(|f| f.id == FieldId::SpFirstName).unwrap();
        assert_eq!((sp_first.get)(&ri, &RowAddr::default()), None, "Sp* get is None without a spouse");
        assert_eq!(
            (sp_first.set)(&mut ri, &RowAddr::default(), FieldValue::Text("Pat".into())),
            Err(SetError::NoSuchRow),
            "Sp* set refuses without a spouse"
        );
        assert!(!(present)(&ri));
        (create)(&mut ri);
        assert!((present)(&ri));
        (sp_first.set)(&mut ri, &RowAddr::default(), FieldValue::Text("Pat".into())).unwrap();
        assert_eq!((sp_first.get)(&ri, &RowAddr::default()), Some(FieldValue::Text("Pat".into())));
    }

    #[test]
    fn secret_fields_mask_and_never_leak_digits() {
        let mut ri = fresh_single();
        let tp = section(SectionId::Taxpayer);
        let ssn = tp.fields.iter().find(|f| f.id == FieldId::TpSsn).unwrap();
        // Empty storage → Empty view.
        assert_eq!((ssn.get)(&ri, &RowAddr::default()), Some(FieldValue::Secret(SecretView::Empty)));
        // Raw entry is stored verbatim (canonicalization is Task 8's parse); get masks it.
        (ssn.set)(&mut ri, &RowAddr::default(), FieldValue::SecretEntry("123456789".into())).unwrap();
        assert_eq!(ri.header.taxpayer.ssn, "123456789");
        let FieldValue::Secret(SecretView::Set { masked }) =
            (ssn.get)(&ri, &RowAddr::default()).unwrap()
        else {
            panic!("expected a Set secret view")
        };
        assert_eq!(masked, "***-**-6789");
        assert!(!masked.contains("12345"), "masked must not leak leading digits");
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
        (ippin.set)(&mut ri, &RowAddr::default(), FieldValue::SecretEntry("112233".into())).unwrap();
        assert_eq!(ri.header.ip_pin.as_deref(), Some("112233"));
        let FieldValue::Secret(SecretView::Set { masked }) =
            (ippin.get)(&ri, &RowAddr::default()).unwrap()
        else {
            panic!("expected a Set secret view for ip_pin")
        };
        assert_eq!(masked, "***-**-2233");
    }

    #[test]
    fn schedule_a_registry_driven_fields_delegate_to_core_registries() {
        use btctax_core::tax::questions::{
            QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
        };
        let sa = section(SectionId::ScheduleA);
        let salt = sa.fields.iter().find(|f| f.id == FieldId::SaSaltUseSalesTax).unwrap();
        let mortgage = sa.fields.iter().find(|f| f.id == FieldId::SaMortgageAllUsed).unwrap();
        assert_eq!(salt.kind, FieldKind::TriState);
        assert_eq!(mortgage.kind, FieldKind::TriState);

        // SALT delegates to SKIPPABLE_QUESTIONS::SalesTaxElection.
        let salt_entry =
            SKIPPABLE_QUESTIONS.iter().find(|e| e.id == SkippableId::SalesTaxElection).unwrap();
        let mut ri = fresh_single();
        ri.schedule_a = Some(Default::default());
        (salt.set)(&mut ri, &RowAddr::default(), FieldValue::TriState(Some(true))).unwrap();
        assert_eq!((salt_entry.get_bool)(&ri), Some(true), "SALT set delegates to the registry");
        assert_eq!((salt.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));
        assert_eq!((salt.live)(&ri), (salt_entry.live)(&ri), "SALT live comes from the registry");

        // Mortgage delegates to FORM_QUESTIONS::MortgageAllUsedToBuyBuildImprove.
        let m_entry = FORM_QUESTIONS
            .iter()
            .find(|e| e.id == QuestionId::MortgageAllUsedToBuyBuildImprove)
            .unwrap();
        ri.schedule_a.as_mut().unwrap().mortgage_interest_1098 = dec!(1); // make the mortgage question live
        (m_entry.set)(&mut ri, true);
        assert_eq!((mortgage.get)(&ri, &RowAddr::default()), Some(FieldValue::TriState(Some(true))));
        assert_eq!(
            (mortgage.live)(&ri),
            (m_entry.live)(&ri),
            "mortgage live comes from the registry gate"
        );
    }
}
