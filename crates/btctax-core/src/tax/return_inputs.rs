//! Full-return v1 **input model** (Phase 1): `ReturnInputs` — the per-year, offline, line-item + PII +
//! payments surface for a Common W-2 household. Stored as JSON in a new `return_inputs` side-table
//! (mirroring `tax_profile`), inside the encrypted vault.
//!
//! **Additive** (SPEC_full_return §2/§4): the crypto **delta** engine and `TaxProfile` stay FROZEN;
//! `ReturnInputs` will *derive* a `TaxProfile` (Phase 2) for the delta path and drives the new absolute
//! assembly + PDF fillers directly. Every optional field is `#[serde(default)]` for forward/backward
//! compatibility (same discipline as `TaxProfile`). All money is `Usd` (exact `Decimal`, cents; NFR5).
use crate::conventions::Usd;
use crate::tax::types::{Carryforward, FilingStatus};
use serde::{Deserialize, Serialize};
use time::Date;

/// Which spouse an item belongs to. Load-bearing for the per-earner Social-Security wage cap (§1402(b),
/// Schedule SE) and the per-person excess-SS credit (§4.9) — box-5 Medicare wages aggregate household-wide
/// but box-3 SS wages are per-earner (deep/02 C4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    #[default]
    Taxpayer,
    Spouse,
}

/// One W-2 box-12 coded amount, captured verbatim (code letter → dollars). Only the inert-allowlist codes
/// `{D,E,F,G,H,S,AA,BB,EE,DD}` are ignorable; any other code refuses (§4.10, spec I1), and Σ of the
/// elective-deferral codes `{D,E,F,G,S}` over §402(g) refuses (spec F3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Box12Entry {
    pub code: String,
    pub amount: Usd,
}

/// One Form W-2. Only CALC/PDF-relevant boxes are typed (SPEC §4.1 / recon-04 §1.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct W2 {
    pub owner: Owner,
    pub employer: String,
    pub box1_wages: Usd,           // → 1040 1a
    pub box2_fed_withheld: Usd,    // → 1040 25a
    #[serde(default)]
    pub box3_ss_wages: Usd, // per-earner SS cap + excess-SS (§4.9)
    #[serde(default)]
    pub box4_ss_withheld: Usd, // → excess-SS credit (§4.9)
    #[serde(default)]
    pub box5_medicare_wages: Usd, // → Form 8959 Part I (household Σ)
    #[serde(default)]
    pub box6_medicare_withheld: Usd, // → Form 8959 Part V → 1040 25c
    #[serde(default)]
    pub box7_ss_tips: Usd,
    #[serde(default)]
    pub box17_state_tax_withheld: Usd, // → Sch A 5a (income-tax election)
    #[serde(default)]
    pub box19_local_tax: Usd, // → Sch A 5a
    #[serde(default)]
    pub box12: Vec<Box12Entry>,
    #[serde(default)]
    pub box13_retirement_plan: bool,
    #[serde(default)]
    pub box8_allocated_tips: Usd, // refuse-guard if > 0 (§4.10)
    #[serde(default)]
    pub box10_dependent_care: Usd, // refuse-guard if > 0 (§4.10)
}

/// Form 1099-INT (SPEC §4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099Int {
    pub payer: String,
    pub box1_interest: Usd, // → 1040 2b / Sch B
    #[serde(default)]
    pub box2_early_withdrawal_penalty: Usd, // → Sch 1 L18
    #[serde(default)]
    pub box3_treasury_interest: Usd, // → 1040 2b
    #[serde(default)]
    pub box4_fed_withheld: Usd, // → 1040 25b
    #[serde(default)]
    pub box6_foreign_tax: Usd, // → §904(j) FTC (§4.7a)
    #[serde(default)]
    pub box8_tax_exempt_interest: Usd, // → 1040 2a (NOT a §1411 add-back)
    #[serde(default)]
    pub box9_private_activity_bond_amt: Usd, // refuse-guard (AMT pref)
}

/// Form 1099-DIV (SPEC §4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099Div {
    pub payer: String,
    pub box1a_ordinary: Usd, // → 1040 3b (INCLUDES 1b)
    #[serde(default)]
    pub box1b_qualified: Usd, // → 1040 3a (preferential)
    #[serde(default)]
    pub box2a_capgain_distr: Usd, // → Sch D L13
    #[serde(default)]
    pub box2b_unrecap_1250: Usd, // refuse-guard (§4.10)
    #[serde(default)]
    pub box2c_section_1202: Usd, // refuse-guard
    #[serde(default)]
    pub box2d_collectibles_28: Usd, // refuse-guard
    #[serde(default)]
    pub box4_fed_withheld: Usd, // → 1040 25b
    #[serde(default)]
    pub box5_section_199a: Usd, // → QBI (§4.5)
    #[serde(default)]
    pub box7_foreign_tax: Usd, // → §904(j) FTC
    #[serde(default)]
    pub box12_exempt_interest_dividends: Usd, // → 1040 2a
    #[serde(default)]
    pub box13_private_activity_amt: Usd, // refuse-guard
}

/// Form 1099-G — unemployment compensation (SPEC §4.3 / I6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099G {
    pub payer: String,
    pub box1_unemployment: Usd, // → Sch 1 L7
    #[serde(default)]
    pub box4_fed_withheld: Usd, // → 1040 25b
}

/// A person on the return (taxpayer or spouse). DOB drives §63(f) age-65 (F3); `blind` is an explicit
/// input (not DOB-derivable, spec I5). SSN stored normalized, rendered masked (security-review item).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
    pub ssn: String,
    #[serde(default)]
    pub ssn_valid_for_employment: bool,
    pub date_of_birth: Option<Date>,
    #[serde(default)]
    pub blind: bool,
    #[serde(default)]
    pub occupation: String,
}

/// A dependent (captured in v1; CTC/ODC is a conservative omission — §3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Dependent {
    pub name: String,
    pub ssn: String,
    #[serde(default)]
    pub ssn_valid_for_employment: bool,
    pub relationship: String,
    pub date_of_birth: Option<Date>,
}

/// 1040 header / PII (vault-only). Fold into the per-year `ReturnInputs` blob (the 1040 is per-year).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HouseholdHeader {
    pub taxpayer: Person,
    #[serde(default)]
    pub spouse: Option<Person>,
    #[serde(default)]
    pub address_street: String,
    #[serde(default)]
    pub address_city: String,
    #[serde(default)]
    pub address_state: String,
    #[serde(default)]
    pub address_zip: String,
    #[serde(default)]
    pub dependents: Vec<Dependent>,
    #[serde(default)]
    pub can_be_claimed_as_dependent_taxpayer: bool,
    #[serde(default)]
    pub can_be_claimed_as_dependent_spouse: bool,
    #[serde(default)]
    pub presidential_fund_taxpayer: bool,
    #[serde(default)]
    pub presidential_fund_spouse: bool,
    #[serde(default)]
    pub ip_pin: Option<String>,
}

/// Schedule C inputs (D-6): business/self-employment crypto income → Sch 1 L3 + Schedule SE. Gross is
/// DERIVED from the ledger's SE-eligible business `crypto_ord` (not typed here). One Sch C in v1; ≥2 SE
/// earners refuse (§4.4a). `net < 0` (loss) refuses (I2).
/// Schedule C line F accounting method (SPEC §4.4a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccountingMethod {
    #[default]
    Cash,
    Accrual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleCInputs {
    pub owner: Owner,
    #[serde(default)]
    pub business_description: String,
    #[serde(default = "default_naics")]
    pub naics_code: String, // line B (NAICS)
    #[serde(default)]
    pub accounting_method: AccountingMethod, // line F
    #[serde(default)]
    pub expenses: Usd,
}
fn default_naics() -> String {
    "999999".to_string()
}
// Manual `Default` so a fresh Schedule C matches the serde default (unclassified NAICS 999999, Cash method).
impl Default for ScheduleCInputs {
    fn default() -> Self {
        Self {
            owner: Owner::default(),
            business_description: String::new(),
            naics_code: default_naics(),
            accounting_method: AccountingMethod::Cash,
            expenses: Usd::ZERO,
        }
    }
}

/// §170(b) charitable ceiling class (deep/04 6-class; ST-crypto = 50%, not 60%). Crypto donations flow
/// from the ledger's computed §170(e) deduction (LT → `CapGainProp30`, ST → `OrdinaryProp50`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharitableClass {
    /// Cash → 50%-org (60% ceiling).
    Cash60,
    /// Cash → non-50%-org (30% ceiling).
    Cash30,
    /// LT capital-gain property (incl. LT crypto) FMV → 50%-org (30% ceiling).
    CapGainProp30,
    /// Capital-gain property → non-50%-org (20% ceiling).
    CapGainProp20,
    /// Ordinary-income/basis property (incl. ST crypto §170(e)) → 50%-org (50% ceiling).
    OrdinaryProp50,
    /// Ordinary property → non-50%-org (30% ceiling).
    OrdinaryProp30,
}

/// A current-year non-crypto charitable gift (SPEC §4.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharitableGift {
    pub class: CharitableClass,
    pub amount: Usd,
}

/// A §170(d)(1) charitable carryover item, tagged by class + vintage (5-year expiry; oldest-first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharitableCarryItem {
    pub class: CharitableClass,
    pub amount: Usd,
    pub origin_year: i32,
}

/// Schedule A inputs (SPEC §4.6). SALT honors the §164(b)(5) income-OR-sales either/or (R2-I4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScheduleAInputs {
    #[serde(default)]
    pub medical: Usd,
    /// §164(b)(5) election: `true` → 5a = sales-tax amount only; `false` → income taxes only.
    #[serde(default)]
    pub salt_use_sales_tax: bool,
    #[serde(default)]
    pub salt_sales_tax_amount: Usd, // used iff `salt_use_sales_tax`
    #[serde(default)]
    pub salt_state_estimated_payments: Usd, // income-tax path
    #[serde(default)]
    pub salt_prior_year_balance_paid: Usd, // income-tax path
    #[serde(default)]
    pub salt_real_estate: Usd, // 5b
    #[serde(default)]
    pub salt_personal_property: Usd, // 5c
    #[serde(default)]
    pub mortgage_interest_1098: Usd, // 8a only
    #[serde(default)]
    pub charitable: Vec<CharitableGift>, // non-crypto; crypto flows from the ledger
}

/// The enumerated minimal Schedule 1 surface (SPEC §4.4 / BLOCKER G1). Only these lines exist; anything
/// else is refused. L3 (Sch C), L7 (unemployment), L15 (½-SE), L18 (early-withdrawal) are DERIVED
/// (not fields here); L1 is user-attested; L21 is a worksheet input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Schedule1Inputs {
    /// L1 — taxable portion of a state/local refund (user attests; §111 worksheet not modeled).
    #[serde(default)]
    pub state_refund_taxable: Usd,
    /// L21 — student-loan interest PAID (the $2,500-cap/MAGI-phaseout worksheet runs in Phase 2/4).
    #[serde(default)]
    pub student_loan_interest_paid: Usd,
    /// An IRA deduction claimed → refuses in v1 (the phase-out worksheet is a follow-on, I3).
    #[serde(default)]
    pub ira_deduction_claimed: Usd,
    /// HSA present → refuses (couples to Form 8889).
    #[serde(default)]
    pub hsa_present: bool,
}

/// Estimated/extension/other payments (SPEC §4.8). Withholding (25a/25b/25c) is summed from the W-2/1099
/// `Vec`s at derivation time, not duplicated here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Payments {
    #[serde(default)]
    pub estimated_tax_payments: Usd, // → 1040 26
    #[serde(default)]
    pub extension_payment: Usd, // → Sch 3 L10
    #[serde(default)]
    pub other_withholding: Usd, // → 1040 25c (warned)
}

/// QBI inputs (SPEC §4.5 / audit I3 — no manual override; auto from box5). REIT/PTP carryforward persists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QbiInputs {
    #[serde(default)]
    pub reit_ptp_carryforward_in: Usd,
}

/// Standard-vs-itemized election (§63(e)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemizeElection {
    /// Take the larger of standard vs Schedule A (the default).
    #[default]
    Auto,
    /// §63(e): elect to itemize even if smaller.
    ForceItemize,
}

/// The full-return household inputs for one tax year — persisted as JSON in the `return_inputs`
/// side-table (year PRIMARY KEY). SPEC_full_return §4.
///
/// `Default` is impl'd manually (not derived) because the frozen `FilingStatus` has no `Default`;
/// a defaulted `ReturnInputs` is Single with all-empty line items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnInputs {
    pub filing_status: FilingStatus,
    #[serde(default)]
    pub header: HouseholdHeader,
    #[serde(default)]
    pub w2s: Vec<W2>,
    #[serde(default)]
    pub int_1099: Vec<Form1099Int>,
    #[serde(default)]
    pub div_1099: Vec<Form1099Div>,
    #[serde(default)]
    pub g_1099: Vec<Form1099G>,
    #[serde(default)]
    pub schedule_c: Option<ScheduleCInputs>,
    #[serde(default)]
    pub schedule_a: Option<ScheduleAInputs>, // None ⇒ standard deduction
    #[serde(default)]
    pub itemize_election: ItemizeElection,
    /// REQUIRED iff MFS: does the spouse itemize? `None` ⇒ fail-loud (§63(c)(6)/G15).
    #[serde(default)]
    pub mfs_spouse_itemizes: Option<bool>,
    #[serde(default)]
    pub sch1: Schedule1Inputs,
    #[serde(default)]
    pub payments: Payments,
    #[serde(default)]
    pub capital_loss_carryforward_in: Carryforward,
    #[serde(default)]
    pub charitable_carryover_in: Vec<CharitableCarryItem>,
    #[serde(default)]
    pub qbi: QbiInputs,
    /// Schedule B Part III — required when Sch B files; `None` ⇒ fail-loud (I7).
    #[serde(default)]
    pub foreign_accounts: Option<bool>,
    /// `Some(true)` ⇒ refuse (Form 3520, R2-I3).
    #[serde(default)]
    pub foreign_trust: Option<bool>,
    #[serde(default)]
    pub foreign_country_names: String,
}

impl Default for ReturnInputs {
    fn default() -> Self {
        Self {
            filing_status: FilingStatus::Single,
            header: HouseholdHeader::default(),
            w2s: Vec::new(),
            int_1099: Vec::new(),
            div_1099: Vec::new(),
            g_1099: Vec::new(),
            schedule_c: None,
            schedule_a: None,
            itemize_election: ItemizeElection::Auto,
            mfs_spouse_itemizes: None,
            sch1: Schedule1Inputs::default(),
            payments: Payments::default(),
            capital_loss_carryforward_in: Carryforward::default(),
            charitable_carryover_in: Vec::new(),
            qbi: QbiInputs::default(),
            foreign_accounts: None,
            foreign_trust: None,
            foreign_country_names: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// A minimal `ReturnInputs` round-trips through JSON, and `#[serde(default)]` lets a sparse blob
    /// (only the required `filing_status`) deserialize — the forward/backward-compat discipline.
    #[test]
    fn returninputs_json_roundtrip_and_sparse_defaults() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                employer: "ACME".into(),
                box1_wages: dec!(82000),
                box2_fed_withheld: dec!(9100),
                box5_medicare_wages: dec!(82000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&ri).unwrap();
        assert_eq!(serde_json::from_str::<ReturnInputs>(&json).unwrap(), ri);

        // Sparse: a blob with only filing_status deserializes (every other field defaults).
        // serde uses the exact FilingStatus variant names (no rename) — "Single", "Mfj", …
        let sparse: ReturnInputs = serde_json::from_str(r#"{"filing_status":"Single"}"#).unwrap();
        assert_eq!(sparse.filing_status, FilingStatus::Single);
        assert!(sparse.w2s.is_empty());
        assert!(sparse.schedule_a.is_none());
        assert_eq!(sparse.itemize_election, ItemizeElection::Auto);
        assert!(sparse.foreign_accounts.is_none()); // tri-state stays unknown
    }

    /// The Schedule C default NAICS is the "unclassified" 999999.
    #[test]
    fn schedule_c_default_naics() {
        let sc = ScheduleCInputs::default();
        assert_eq!(sc.naics_code, "999999");
    }
}
