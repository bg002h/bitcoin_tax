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
    pub box1_wages: Usd,        // → 1040 1a
    pub box2_fed_withheld: Usd, // → 1040 25a
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
/// input (not DOB-derivable, spec I5). SSN is stored AS ENTERED and rendered masked by `mask_pii` (the
/// security-load-bearing half); canonicalization to NNN-NN-NNNN is deferred to P6 (see FOLLOWUPS).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
    pub ssn: String,
    pub date_of_birth: Option<Date>,
    /// **1040 line 12a / Schedule 1-A Part V — the §63(f) and §151(d)(5) death carve-out.**
    ///
    /// i1040gi (2024, Standard Deduction): *"If your spouse was born before January 2, 1960, but died
    /// in 2024 before reaching age 65, **don't check the box** that says 'Spouse was born before
    /// January 2, 1960.' A person is considered to reach age 65 on the day before the person's 65th
    /// birthday."* The 2025 instructions repeat it for Schedule 1-A Part V with the IRS's own boundary
    /// pair: born 1960-02-14, died 2025-02-13 qualifies; died 2025-02-12 does not.
    ///
    /// ★ `None` means "no death recorded", which `is_aged` treats as *did not die* — so on its own it
    /// would grant a box the filer may not be entitled to. That is why
    /// [`HouseholdHeader::taxpayer_died_during_year`] exists: the GATE carries the answered-ness, this
    /// field carries the date. See `FOLLOWUPS.md` §G-9 — a live defect through v0.14.0.
    #[serde(default)]
    pub date_of_death: Option<Date>,
    /// §63(f) additional standard deduction for blindness — a class-(B) TRI-STATE (P9 §2.2). `None` = never
    /// asked; the advisory fires on `None` (never on `Some(false)`), so a filer who told us they are sighted
    /// is not nagged. A bare `bool` here was the D-8 shape: never-asked indistinguishable from answered-No.
    #[serde(default)]
    pub blind: Option<bool>,
    #[serde(default)]
    pub occupation: String,
}

/// A dependent (captured in v1; CTC/ODC is a conservative omission — §3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Dependent {
    pub name: String,
    pub ssn: String,
    pub relationship: String,
    pub date_of_birth: Option<Date>,
}

/// 1040 header / PII (vault-only). Fold into the per-year `ReturnInputs` blob (the 1040 is per-year).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HouseholdHeader {
    /// `#[serde(default)]` deliberately: the taxpayer's PII is captured LATER (`btctax set-pii`) and is
    /// only enforced at export (an SSN-less return refuses there, not at import). Without the default, any
    /// partial `[header]` table — e.g. one that answers only the dependent flag — failed with
    /// "missing field `taxpayer`", which D-8 turned from a curiosity into a wall, since `[header]` is now
    /// mandatory on every return.
    #[serde(default)]
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
    /// 1040 "Someone can claim: **You** as a dependent" — a TRI-STATE, and it must stay one.
    ///
    /// `None` = **never asked**, and it REFUSES (`DependentStatusUnanswered`). A bare `bool` here was a
    /// live defect: it made "unanswered" and "answered No" the same value, so an unasked filer silently
    /// got the FULL basic standard deduction instead of the §63(c)(5) dependent floor (understating tax),
    /// slipped past the §1(g)/Form-8615 kiddie-tax refusal, AND printed the 1040's checkbox unchecked —
    /// a false statement on a filed form. There is no safe default: guessing `false` understates, and
    /// guessing `true` overstates. Only the filer knows. See `design/SPEC_dependent_flag.md`.
    #[serde(default)]
    pub can_be_claimed_as_dependent_taxpayer: Option<bool>,
    /// 1040 "Someone can claim: **Your spouse** as a dependent" — tri-state, same reasoning. Required
    /// only when a spouse is actually on this return; `Some(true)` refuses (`DependentSpouseUnsupported`).
    #[serde(default)]
    pub can_be_claimed_as_dependent_spouse: Option<bool>,
    #[serde(default)]
    pub presidential_fund_taxpayer: bool,
    #[serde(default)]
    pub presidential_fund_spouse: bool,
    /// **The §G-9 gate (taxpayer): "did the taxpayer die during the tax year?"** A class-(A)
    /// DECLARATION, because an unanswered death **grants** a deduction — the opposite direction from
    /// [`Person::blind`], a class-(B) benefit claim whose `None` merely forgoes one. `None` refuses.
    ///
    /// It sits HERE rather than on [`Person`], alongside the other per-person declarations
    /// (`can_be_claimed_as_dependent_*`, `presidential_fund_*`), for a mechanical reason as well as a
    /// stylistic one: `Person`'s name and SSN are serde-REQUIRED, so a gate on `Person` would force a
    /// complete `[header.taxpayer]` table into every inputs TOML that wants to answer it.
    ///
    /// The DATE is [`Person::date_of_death`] — next to `date_of_birth`, where dates belong.
    #[serde(default)]
    pub taxpayer_died_during_year: Option<bool>,
    /// **The §G-9 gate (spouse)** — same rule, live only on a return that carries a spouse `Person`.
    #[serde(default)]
    pub spouse_died_during_year: Option<bool>,
    #[serde(default)]
    pub ip_pin: Option<String>,
}

/// Schedule C line F accounting method (SPEC §4.4a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccountingMethod {
    #[default]
    Cash,
    Accrual,
}

/// Schedule C inputs (D-6): business/self-employment crypto income → Sch 1 L3 + Schedule SE. Gross is
/// DERIVED from the ledger's SE-eligible business `crypto_ord` (not typed here). One Sch C in v1; ≥2 SE
/// earners refuse (§4.4a). `net < 0` (loss) refuses (I2).
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

/// Whether a carryover-IN value was entered by the user (`income import`) or computed by a prior report's
/// write-back (SPEC §4 R3-M6). A report write-back **overwrites a Computed** carryover-in silently but
/// **refuses to silently overwrite a User** one (warn + `--force`). Defaults to `User` for back-compat:
/// every pre-existing (imported) carryover is user-entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CarryProvenance {
    #[default]
    User,
    Computed,
}

/// A §170(d)(1) charitable carryover item, tagged by class + vintage (5-year expiry; oldest-first) +
/// provenance (§4 R3-M6). `provenance` is `#[serde(default)]` so existing blobs load as `User`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharitableCarryItem {
    pub class: CharitableClass,
    pub amount: Usd,
    pub origin_year: i32,
    #[serde(default)]
    pub provenance: CarryProvenance,
}

/// Schedule A inputs (SPEC §4.6). SALT honors the §164(b)(5) income-OR-sales either/or (R2-I4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScheduleAInputs {
    #[serde(default)]
    pub medical: Usd,
    /// §164(b)(5) election — a class-(B) TRI-STATE (P9 §2.2). `Some(true)` → 5a = sales-tax amount only;
    /// `Some(false)` → income taxes only; `None` = never asked (the `SalesTaxElectionNotAsked` advisory fires
    /// on `None`, scoped to returns that carry a Schedule A).
    #[serde(default)]
    pub salt_use_sales_tax: Option<bool>,
    #[serde(default)]
    pub salt_sales_tax_amount: Usd, // used iff `salt_use_sales_tax == Some(true)`
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
    /// §163(h)(3)(F) mixed-use mortgage — a class-(A) DECLARATION (P9 §2.7), live when this Schedule A carries
    /// mortgage interest. `None` ⇒ refuse (`MixedUseMortgageUnanswered`); `Some(false)` ⇒ 8a is zeroed, the
    /// line-8 box is checked, and `MixedUseMortgageNotAllocated` advises (v1 cannot do the Pub. 936 split);
    /// `Some(true)` ⇒ full 8a, box unchecked.
    #[serde(default)]
    pub mortgage_all_used_to_buy_build_improve: Option<bool>,
    /// **Form 6251 line 3 — the AMT qualified-dwelling declaration.** i6251 p.8: "If you deducted home
    /// mortgage interest on Schedule A for a dwelling that isn't a principal residence (within the
    /// meaning of section 121) or qualified dwelling for AMT, include that deducted interest on line 3.
    /// A qualified dwelling for AMT is a house, apartment, condominium, or mobile home not used on a
    /// transient basis. A qualified dwelling for AMT doesn't include house boats and recreational
    /// vehicles."
    ///
    /// A class-(A) DECLARATION, phrased so `true` is the AMT-neutral answer. `None` = never asked ⇒
    /// refuse. `Some(false)` = "not an AMT-qualified dwelling" ⇒ ALSO refuse: v1 does not model the
    /// line-3 add-back, and computing without it would UNDERSTATE the tax.
    #[serde(default)]
    pub mortgage_dwelling_is_amt_qualified: Option<bool>,
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
    /// §223 HSA ACTIVITY — a class-(A) DECLARATION (P9 §2.4), live always. RENAMED from `hsa_present`: the old
    /// field asked "do you hold an HSA?"; this asks whether a Form 8889 *trigger* fired (a contribution by
    /// anyone, a distribution, a testing-period inclusion, or an inheritance). `None` ⇒ refuse
    /// (`HsaActivityUnanswered`); `Some(true)` ⇒ refuse unsupported; `Some(false)` ⇒ a dormant holder proceeds.
    #[serde(default)]
    pub hsa_activity: Option<bool>,
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

/// QBI inputs (SPEC §4.5 / audit I3 — no manual override; auto from box5). REIT/PTP carryforward persists,
/// with a provenance flag for the §4 R3-M6 write-back precedence (default `User`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QbiInputs {
    #[serde(default)]
    pub reit_ptp_carryforward_in: Usd,
    #[serde(default)]
    pub reit_ptp_carryforward_in_provenance: CarryProvenance,
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
    /// ★★★ **§G-15 — the tax year these inputs are FOR.**
    ///
    /// Added because the question registry could not scope a question to a year: `questions.rs`
    /// documented it verbatim — *"`live` receives only `&ReturnInputs`, which carries no tax year, so
    /// it CANNOT be scoped"* — which forced `HasIncomeExclusion` (a TY2025 MAGI question) to ship
    /// ALWAYS LIVE, asking TY2024 filers a TY2025 question. That workaround needed a bespoke
    /// neutrality proof, and **Schedule 1-A Part IV has none**: asking a TY2024 filer about a
    /// deduction that did not exist in 2024 is answering a question with no TY2024 legal meaning.
    ///
    /// ★ `live` deliberately KEEPS its one-argument signature and reads this field, rather than
    /// becoming `live(year, &ri)` — a year passed alongside invites a caller to pass one that
    /// disagrees with the row's own origin, which is a second copy of the truth in flight.
    ///
    /// ★★ **`0` means NOT STATED**, and it is not a usable year: the storage boundary stamps this
    /// from the row key on read and refuses a disagreement on write, so an in-memory value and the
    /// row it came from can never diverge. `Default` yields `0` because a defaulted `ReturnInputs` is
    /// a test convenience that must not fabricate a year any more than it fabricates an answer.
    #[serde(default)]
    pub tax_year: i32,
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
    /// **Form 6251 line 2k — the AMT capital-loss-carryover declaration.** Line 2k is "Disposition of
    /// property (difference between AMT and regular tax gain or loss)", and i6251 directs any Form
    /// 8949 / Schedule D / 4684 / 4797 adjustment for the activity there rather than to line 3.
    /// btctax carries only the REGULAR-tax carryforward and cannot know whether an AMT twin diverges.
    ///
    /// A class-(A) DECLARATION, phrased so `true` is the AMT-neutral answer. `None` ⇒ refuse (never
    /// asked); `Some(false)` — "my AMT carryover differs" — ⇒ ALSO refuse, because v1 models no
    /// divergence and proceeding would UNDERSTATE the tax.
    ///
    /// Lives on `ReturnInputs`, not inside [`Carryforward`], because that type is a shared value also
    /// used by `TaxProfile`; the declaration is a property of the RETURN, not of the amount.
    #[serde(default)]
    pub amt_carryover_same_as_regular: Option<bool>,

    /// Form 6251 **line 2l** — "is the depreciation inside my Schedule C expenses the same for the AMT?"
    ///
    /// Lives on `ReturnInputs` rather than on [`ScheduleCInputs`] to match its line-2k sibling above: the
    /// declaration is a property of the RETURN, not of the business. (It is also why the liveness
    /// predicate, not the field's location, is what ties it to Schedule C.)
    #[serde(default)]
    pub amt_depreciation_same_as_regular: Option<bool>,
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
    /// 1040 header "you were a dual-status alien" — a class-(A) DECLARATION (P9 §2.5), live always. A single
    /// box asserting a fact whose unchecked state we print today from the MFS coupling alone; `None` ⇒ refuse
    /// (`DualStatusAlienUnanswered`), `Some(true)` ⇒ refuse unsupported (§63(c)(6)(B): NRA standard deduction
    /// is zero), `Some(false)` ⇒ proceed.
    #[serde(default)]
    pub dual_status_alien: Option<bool>,

    // ── §164(b)(7)(B)(iv) / Schedule 1-A Part I — the MAGI add-backs ──────────────────────────────
    //
    // ★ The SAME four amounts serve two forms. The §164(b) SALT worksheet adds them at its lines
    //   3a–3d, and Schedule 1-A Part I adds them at its lines 2a–2d, because both use the statute's
    //   modified AGI: "adjusted gross income increased by any amount excluded from gross income under
    //   section 911, 931, or 933." One quantity, five phase-outs (SALT plus Schedule 1-A's four).
    //
    // ★ `Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
    //   defaulting to 0 cannot tell "zero because the filer has none" from "zero because nobody
    //   asked" — and those differ in direction: an unasked add-back UNDERSTATES MAGI, which RAISES
    //   the SALT and Schedule 1-A deductions. That is a default in the filer's favour that
    //   understates tax, so it is not the class-(B) forgone-benefit case `medical` is.
    //   `None` refuses, but only where a form actually needs the number — see `SaltLimitation`.
    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I — the exclusion GATE.** A class-(A) DECLARATION:
    /// *"Did you exclude any income from gross income under §911 (foreign earned income / housing),
    /// §931 (American Samoa) or §933 (Puerto Rico)?"*
    ///
    /// ★ The gate carries the answered-ness, not the amounts, and that is deliberate. `Option<bool>`
    /// is a leaf the classifier **forbids** `_` on, so it cannot be added without a human classifying
    /// it — whereas `Option<Usd>` is a scalar the `_` rule permits, which would make this convention
    /// again. `None` = never asked ⇒ refused where a form needs MAGI. `Some(false)` ⇒ all four
    /// amounts are zero. `Some(true)` ⇒ the amounts below are the filer's answers.
    #[serde(default)]
    pub has_income_exclusion: Option<bool>,
    /// **Schedule 1-A line 2a / SALT worksheet line 3a** — "Enter any income from Puerto Rico that
    /// you excluded." (§933.) Meaningful iff [`Self::has_income_exclusion`] is `Some(true)`.
    #[serde(default)]
    pub excluded_puerto_rico_income: Usd,
    /// **Schedule 1-A line 2b / SALT worksheet line 3b** — "Enter the amount from Form 2555, line
    /// 45." (§911 foreign earned income exclusion.)
    #[serde(default)]
    pub form_2555_line45: Usd,
    /// **Schedule 1-A line 2c / SALT worksheet line 3c** — "Enter the amount from Form 2555, line
    /// 50." (§911 housing exclusion.)
    #[serde(default)]
    pub form_2555_line50: Usd,
    /// **Schedule 1-A line 2d / SALT worksheet line 3d** — "Enter the amount from Form 4563, line
    /// 15." (§931 American Samoa exclusion.)
    #[serde(default)]
    pub form_4563_line15: Usd,
}

impl ReturnInputs {
    /// §164(b)(7)(B)(iv) **modified** adjusted gross income — AGI plus the §911/931/933 exclusions.
    /// The same quantity Schedule 1-A Part I line 3 computes.
    ///
    /// `None` when any add-back was never asked, so a caller that genuinely needs MAGI refuses rather
    /// than silently treating an unasked exclusion as zero. Callers that never need it (TY2024's flat
    /// SALT cap) never call this.
    pub fn modified_agi(&self, agi: Usd) -> Option<Usd> {
        match self.has_income_exclusion {
            None => None, // never asked — a caller that needs MAGI must refuse, not assume zero
            Some(false) => Some(agi),
            Some(true) => Some(
                agi + self.excluded_puerto_rico_income
                    + self.form_2555_line45
                    + self.form_2555_line50
                    + self.form_4563_line15,
            ),
        }
    }
}

impl Default for ReturnInputs {
    fn default() -> Self {
        Self {
            // ★ §G-15: `0` = NOT STATED. Default() is a test convenience and must not fabricate a
            // tax year any more than it fabricates an answer.
            tax_year: 0,
            // §911/931/933 add-backs: `None` = never asked. Default() is a TEST convenience, so
            // it must not fabricate an answer — see the field docs.
            has_income_exclusion: None,
            excluded_puerto_rico_income: Usd::ZERO,
            form_2555_line45: Usd::ZERO,
            form_2555_line50: Usd::ZERO,
            form_4563_line15: Usd::ZERO,
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
            amt_carryover_same_as_regular: None,
            amt_depreciation_same_as_regular: None,
            charitable_carryover_in: Vec::new(),
            qbi: QbiInputs::default(),
            foreign_accounts: None,
            foreign_trust: None,
            foreign_country_names: String::new(),
            dual_status_alien: None,
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

    /// ★ P9 step 1 — the answered-ness fields are tri-state, and DEFAULT TO UNANSWERED (`None`), never to a
    /// fabricated "No". A bare `bool` here is the D-8 defect (never-asked == answered-No). The rename
    /// `hsa_present → hsa_activity` and the two NEW declarations are asserted here so the shape is pinned by a
    /// test, not just by the compiler.
    #[test]
    fn p9_answeredness_fields_default_to_unanswered() {
        let p = Person::default();
        assert_eq!(
            p.blind, None,
            "§63(f) blindness is a tri-state; unasked is None, not false"
        );

        let a = ScheduleAInputs::default();
        assert_eq!(
            a.salt_use_sales_tax, None,
            "§164(b)(5) election is a tri-state; unasked is None"
        );
        assert_eq!(
            a.mortgage_all_used_to_buy_build_improve, None,
            "§163(h)(3)(F) mixed-use question is a NEW declaration; unasked is None"
        );

        let s1 = Schedule1Inputs::default();
        assert_eq!(
            s1.hsa_activity, None,
            "hsa_present RENAMED to hsa_activity (a different question — §223 triggers, not mere holding); unasked is None"
        );

        let ri = ReturnInputs::default();
        assert_eq!(
            ri.dual_status_alien, None,
            "§63(c)(6)(B) dual-status is a NEW declaration, live always; unasked is None"
        );
    }
}
