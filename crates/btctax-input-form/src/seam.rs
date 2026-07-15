//! The data seam (spec §4, §5.7): a render model + an Edit stream both renderers consume.
use btctax_core::conventions::Usd;
use btctax_core::tax::return_inputs::ReturnInputs;
use serde::{Deserialize, Serialize};
use std::fmt;
use time::Date;

/// A path of indices to a row; empty for singletons; ≤ 2 today (`[w2_i, box12_i]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RowAddr(pub Vec<usize>);

/// Stable section identity — the wire contract; NEVER a Vec index (spec §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionId {
    ReturnOptions, Taxpayer, Spouse, Address, Dependents,
    W2s, W2Box12, ScheduleA, ScheduleACharitable, Payments,
    Declarations, Skippables,
}

/// Stable field identity. One per leaf across the v1 sections (spec §5.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldId {
    // ReturnOptions
    FilingStatus, ItemizeElection,
    // Taxpayer / Spouse (Person + header.ip_pin)
    TpFirstName, TpLastName, TpSsn, TpOccupation, TpPresidentialFund, IpPin,
    SpFirstName, SpLastName, SpSsn, SpOccupation, SpPresidentialFund,
    // Address
    AddrStreet, AddrCity, AddrState, AddrZip,
    // Dependents (per row)
    DepName, DepSsn, DepRelationship, DepDob,
    // W2 (per row)
    W2Owner, W2Employer, Box1Wages, Box2FedWh, Box3SsWages, Box4SsWh,
    Box5MedWages, Box6MedWh, Box7SsTips, Box17StateWh, Box19LocalTax,
    Box8AllocTips, Box10DepCare,
    // W2 box 12 (per row)
    Box12Code, Box12Amount,
    // Schedule A
    SaMedical, SaSaltRealEstate, SaSaltPersonalProp, SaSaltStateEst,
    SaSaltPriorYear, SaSaltSalesTaxAmt, SaMortgage1098,
    SaSaltUseSalesTax, SaMortgageAllUsed,
    // Schedule A charitable (per row)
    CharClass, CharAmount,
    // Payments
    PayEstimated, PayExtension, PayOtherWh,
    // Declarations (from FORM_QUESTIONS) + the 7b country text
    DeclDependentTaxpayer, DeclDependentSpouse, DeclMfsSpouseItemizes,
    DeclForeignAccounts, DeclForeignTrust, DeclHsaActivity, DeclDualStatusAlien,
    ForeignCountryNames,
    // Skippables (from SKIPPABLE_QUESTIONS); SALT election = SaSaltUseSalesTax in Schedule A above
    BlindTaxpayer, BlindSpouse, DobTaxpayer, DobSpouse,
}

/// The value shape of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind { Money, Text, Bool, TriState, Date, Enum(&'static [&'static str]), Secret }

/// A field value crossing the seam (spec §4/§5.7). Owned (serde), so it is the web wire.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Money(Usd),
    Text(String),
    Bool(bool),
    TriState(Option<bool>),
    Date(Option<Date>),
    Choice(String),          // an Enum choice by its stable name
    Secret(SecretView),      // OUTBOUND only (get) — presence, never digits
    SecretEntry(String),     // INBOUND only (set) — masked Debug; get never returns it
}

/// A secret's presence, never its digits (spec §4/§5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretView { Empty, Set { masked: String } }

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::SecretEntry(_) => write!(f, "SecretEntry(***)"),   // never leak digits
            FieldValue::Money(v) => write!(f, "Money({v})"),
            FieldValue::Text(s) => write!(f, "Text({s:?})"),
            FieldValue::Bool(b) => write!(f, "Bool({b})"),
            FieldValue::TriState(t) => write!(f, "TriState({t:?})"),
            FieldValue::Date(d) => write!(f, "Date({d:?})"),
            FieldValue::Choice(c) => write!(f, "Choice({c:?})"),
            FieldValue::Secret(s) => write!(f, "Secret({s:?})"),
        }
    }
}

/// A leaf field (spec §5.2). Accessors are monomorphic over `(&ReturnInputs, RowAddr)` — the row type never
/// appears (spec §4). Secret `get` returns presence; `set` accepts only `SecretEntry`.
pub struct Field {
    pub id: FieldId,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: FieldKind,
    pub live: fn(&ReturnInputs) -> bool,
    pub get: fn(&ReturnInputs, &RowAddr) -> Option<FieldValue>,
    pub set: fn(&mut ReturnInputs, &RowAddr, FieldValue) -> Result<(), SetError>,
}

/// A section: a singleton, an optional-singleton (create/delete), or a repeating group (spec §5.1).
pub struct Section {
    pub id: SectionId,
    pub title: &'static str,
    pub kind: SectionKind,
    pub fields: &'static [Field],
}

pub enum SectionKind {
    Singleton,
    OptionalSingleton {
        present: fn(&ReturnInputs) -> bool,
        create: fn(&mut ReturnInputs),
        delete: fn(&mut ReturnInputs),
    },
    Repeating {
        len: fn(&ReturnInputs, &RowAddr) -> usize,
        add: fn(&mut ReturnInputs, &RowAddr),
        remove: fn(&mut ReturnInputs, &RowAddr),
    },
}

/// An edit from a renderer (spec §5.7). Serde-serializable — the web wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    SetField { id: FieldId, addr: RowAddr, value: FieldValue },
    ClearField { id: FieldId, addr: RowAddr },
    AddRow { section: SectionId, parent: RowAddr },
    RemoveRow { section: SectionId, addr: RowAddr },
    CreateSection { section: SectionId },
    DeleteSection { section: SectionId },
}

/// Where a `RefuseReason` points in the form (spec §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor { Field(FieldId), Section(SectionId), NotInForm { note: &'static str } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError { WrongKind, NoSuchRow, Immutable }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError { NotANumber, Negative, BadDate, BadSsn, BadIpPin, NotAChoice }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError { NotChosenYet, WrongFirstEdit, SetError(SetError), NoSuchSection }

#[cfg(test)]
mod tests {
    use super::*;

    /// The Edit/FieldValue seam serializes losslessly (the web wire, spec §4/M-5).
    #[test]
    fn edit_roundtrips_through_json() {
        let e = Edit::SetField {
            id: FieldId::Box1Wages,
            addr: RowAddr(vec![0]),
            value: FieldValue::Money(rust_decimal_macros::dec!(50000)),
        };
        let j = serde_json::to_string(&e).unwrap();
        let back: Edit = serde_json::from_str(&j).unwrap();
        assert_eq!(e, back);
    }

    /// A SecretView never carries digits; SecretEntry is inbound-only and masks its Debug.
    #[test]
    fn secret_view_is_presence_only_and_entry_masks_debug() {
        assert_eq!(
            SecretView::Set { masked: "***-**-6789".into() },
            SecretView::Set { masked: "***-**-6789".into() }
        );
        let entry = FieldValue::SecretEntry("123456789".into());
        assert!(!format!("{entry:?}").contains("123456789"), "SecretEntry Debug must not leak digits");
    }
}
