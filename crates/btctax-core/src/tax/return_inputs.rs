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
    /// Box b — the employer's **EIN**, and the only thing that can answer §6413(c)'s *"more than one
    /// employer"* test. `None` = never stated.
    ///
    /// ★★★ It is here because its absence was a live **understatement of tax**. i1040gi, Schedule 3
    /// line 11: *"If you, or your spouse if filing a joint return, had **more than one employer** for
    /// 2024 and total wages of more than $168,600 … You can take a credit … for the amount withheld in
    /// excess of $10,453.20. But if **any one employer** withheld more than $10,453.20, you can't claim
    /// the excess on your return."* Two conditions; btctax enforced only the second.
    ///
    /// ★★ [`super::return_1040::excess_social_security`] carried a confident equivalence comment that
    /// was simply wrong — *"a single-employer person nets 0, so the 'requires ≥ 2 employers' rule falls
    /// out naturally"*. It does not: **one employer may issue several W-2s to one person** (a corrected
    /// W-2, a payroll-system change mid-year, separate establishments under one EIN), each under the
    /// per-W-2 cap and summing over it. A filing trial credited **$3,894** to a filer entitled to $0,
    /// turning an $1,085 liability into a $2,809 refund, on a return signed under §6065.
    ///
    /// `employer` is free text and is NOT a substitute — nothing reads it, and two spellings of one
    /// employer are two employers to a string compare.
    #[serde(default)]
    pub ein: Option<String>,
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

/// **Form 1099-B** — broker proceeds, as TOTALS for Schedule D lines 1a and 8a (§G-28/B4).
///
/// ★★★ TOTALS, NEVER LOT-LEVEL, and that is the FORM'S OWN DESIGN rather than a shortcut. Schedule D
/// line 1a reads:
///
/// > *"Totals for all short-term transactions reported on Form 1099-B for which basis was reported to
/// > the IRS and for which you have no adjustments (see instructions). However, if you choose to report
/// > all these transactions on Form 8949, leave this line blank and go to line 1b."*
///
/// Line 8a says the same for long-term. So a filer whose broker reported basis and who needs no
/// adjustment is expressly told they *"aren't required to report these transactions on Form 8949"* —
/// four numbers replace a transaction list. btctax therefore never builds a second lot-level engine for
/// non-crypto securities: the crypto lot engine stays the only one, and these totals arrive already
/// netted from the broker.
///
/// ★★ THE TWO CONDITIONS ARE THE FILER'S TESTIMONY, so they are asked, not assumed — see
/// [`Self::basis_reported_and_no_adjustments`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099B {
    /// The broker, for the filer's own records. Schedule D lines 1a/8a name no payer (Form 8949 would),
    /// so nothing on the printed return reads this — it exists so a filer with three brokers can tell
    /// their three rows apart.
    pub payer: String,
    /// **Schedule D line 1a, column (d)** — *"Proceeds (sales price)"*, short-term.
    #[serde(default)]
    pub short_term_proceeds: Usd,
    /// **Schedule D line 1a, column (e)** — *"Cost (or other basis)"*, short-term.
    #[serde(default)]
    pub short_term_basis: Usd,
    /// **Schedule D line 8a, column (d)** — *"Proceeds (sales price)"*, long-term.
    #[serde(default)]
    pub long_term_proceeds: Usd,
    /// **Schedule D line 8a, column (e)** — *"Cost (or other basis)"*, long-term.
    #[serde(default)]
    pub long_term_basis: Usd,
    /// ★★★ THE GATE. Lines 1a/8a are available ONLY for transactions where **basis was reported to the
    /// IRS** *and* **there are no adjustments**. Anything else belongs on Form 8949 with Box B, C, E or
    /// F checked and PER-TRANSACTION detail — which is exactly the lot-level engine btctax will not
    /// build for non-crypto securities. A row that is not `Some(true)` REFUSES
    /// ([`super::return_refuse::RefuseReason::Form1099BNeedsForm8949`]).
    ///
    /// ★★ Framed as the YES-condition, and both limbs are named in the prompt, because *"widening an
    /// exemption is never the safe edit"*: a filer answering something vaguer would have a `no` on one
    /// limb laundered into a `yes` on both. `None` and `Some(false)` both refuse, so every omission
    /// fails closed.
    ///
    /// ★ It is per-1099-B, not per-return: one broker may report basis on everything while another does
    /// not, and the filer should not have to answer for the worst of them.
    #[serde(default)]
    pub basis_reported_and_no_adjustments: Option<bool>,
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
    /// ★★ §G-20 — i1040gi's remaining two conditions for claiming a spouse's §63(f) aged/blind boxes on
    /// **married filing separately**:
    ///
    /// > *"If your filing status is married filing separately and your spouse was born before January 2,
    /// > 1960, or was blind at the end of 2024, you can check the appropriate box(es) … **if your spouse
    /// > had no income, isn't filing a return, and can't be claimed as a dependent on another person's
    /// > return**."*
    ///
    /// The third condition is [`Self::can_be_claimed_as_dependent_spouse`], already captured. These two
    /// are class (B) — BENEFIT CLAIMS, so silence FORGOES the boxes and never grants them.
    ///
    /// ★★★ **Both must be `Some(true)` (and the dependent flag `Some(false)`) before a single box is
    /// counted.** Every other combination — including any unanswered — forgoes. This is the one place
    /// on the branch where an answer can only ever REDUCE tax, so it fails closed by construction: an
    /// omission here costs the filer a deduction, which is recoverable; a wrong grant understates a
    /// signed return, which is not.
    #[serde(default)]
    pub spouse_had_no_income: Option<bool>,
    /// See [`Self::spouse_had_no_income`] — the second of i1040gi's two uncaptured MFS conditions.
    #[serde(default)]
    pub spouse_not_filing_a_return: Option<bool>,
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
    /// **Schedule C line 1** — gross receipts or sales that did **not** arrive through the Bitcoin
    /// ledger. Consulting, freelance, any non-crypto revenue of the same trade or business.
    ///
    /// ★★★ §G-28/B3 — THIS IS THE FIELD THAT STOPS btctax BEING BITCOIN-ONLY FOR SELF-EMPLOYMENT.
    /// Before it, Schedule C revenue came *exclusively* from `se_net_income(state, year)`, so a filer
    /// with $85,000 of consulting income could not represent it at all. Line 1 is now the SUM: the
    /// ledger's SE-eligible business crypto **plus** this. Everything downstream — net profit,
    /// Schedule SE, §199A QBI — follows from that one line.
    ///
    /// ★★ A PLAIN `Usd` defaulting to zero, and the reason is a guard rather than a convention. A
    /// defaulted zero here is testimony that all business revenue came through the ledger, and that
    /// is the UNDERSTATING direction — so it would need answered-ness were it not already covered.
    /// It is: [`ReturnInputs::other_out_of_scope_income`] is a MANDATORY class-(A) declaration whose
    /// prompt asks about *"a business this tool did not capture, or anything else it never asked
    /// about"*, and a `yes` refuses. A filer with non-ledger receipts must answer it, so they cannot
    /// reach a filed return by silence. ★ If that declaration is ever narrowed, this field must
    /// become `Option<Usd>` in the same commit.
    ///
    /// ★ The LEDGER remains the sole authority for crypto receipts. This is additive and never
    /// replaces it — a filer cannot restate their mined income here (that is §G-28/B5, decided
    /// DO-NOT-BUILD for exactly that reason).
    #[serde(default)]
    pub other_gross_receipts: Usd,
    /// ★★ Line **I** — *"Did you make any payments in 2024 that would require you to file Form(s)
    /// 1099?"* A compliance DECLARATION about the filer's own information-reporting, with real
    /// §6721/§6722 exposure — but no figure on the return reads it, and the form prints no Caution
    /// beside it, so it is class (B): asked, and lawfully skippable
    /// ([`super::questions::SkippableId::ScheduleC1099Required`]).
    ///
    /// `Option`, and the `Option` is load-bearing to the PDF writer: `None` (never asked) and
    /// `Some(false)` (asked, answered no) are DIFFERENT marks on the page — an unwritten pair versus
    /// a checked No box. A printed "No" the filer never gave is fabricated testimony.
    #[serde(default)]
    pub payments_requiring_1099: Option<bool>,
    /// Line **J** — *"If 'Yes,' did you or will you file required Form(s) 1099?"* The form conditions
    /// it on line I, so it is live only when [`Self::payments_requiring_1099`] is `Some(true)`.
    ///
    /// ★ This is the question whose liveness depends on another question's NON-NEUTRAL answer — the
    /// shape that silently broke the registry's own property harness once (see the `is_none()` guards
    /// in `return_refuse.rs`). It is now the live exerciser of those guards.
    #[serde(default)]
    pub will_file_required_1099: Option<bool>,
    /// **Form 8995-A line 4** — *"Allocable share of W-2 wages from the trade, business, or
    /// aggregation"*.
    ///
    /// ★★★ Above the §199A(e)(2) threshold this is half of what decides the deduction: §199A(b)(2)
    /// caps it at the greater of 50% of W-2 wages, or 25% of wages plus 2.5% of UBIA. A sole
    /// proprietor with no employees has **zero**, which caps the deduction at zero — a real answer, and
    /// the reason the old blanket refusal was wrong for that filer.
    ///
    /// `None` = never asked, and it REFUSES above the threshold rather than defaulting: a defaulted
    /// zero would understate the cap for a filer who does pay wages, and a defaulted anything-else
    /// would invent them. Below the threshold nothing reads it.
    #[serde(default)]
    pub qbi_w2_wages: Option<Usd>,
    /// **Form 8995-A line 7** — *"Allocable share of the unadjusted basis immediately after
    /// acquisition (UBIA) of all qualified property"*. The other half of the §199A(b)(2) cap; same
    /// `None`-refuses treatment as [`Self::qbi_w2_wages`].
    #[serde(default)]
    pub qbi_ubia: Option<Usd>,
    /// **Form 8995-A Part I, column (b)** — *"Check if specified service"*.
    ///
    /// ★★★ **Offered always, MANDATORY only where it matters** — the `DonationsHadRestrictions` shape,
    /// not a class-(A) declaration. Above the phase-in range an SSTB's QBI is excluded ENTIRELY
    /// (§199A(d)(3)), so a "no" the filer never gave hands them a deduction the statute denies — an
    /// understatement — and `screen_absolute` demands the answer *there*, where taxable income is known.
    /// **Below** the threshold §199A is the simplified Form 8995, which has **no SSTB checkbox at all**,
    /// so the answer changes nothing and demanding it is a refusal with no purpose.
    ///
    /// ★ A draft made it live-and-mandatory on `schedule_c.is_some()` and it refused **every Schedule C
    /// return at any income**; `below_the_threshold_an_unanswered_sstb_does_not_refuse` is what reds if
    /// that regresses.
    ///
    /// The code once reasoned that every Schedule C here is crypto mining and therefore not an SSTB;
    /// that is true of mining and stops being true the moment a filer can state their own gross
    /// receipts (B3). Asking is transcription: the checkbox is on the form.
    #[serde(default)]
    pub is_sstb: Option<bool>,
    /// **Form 8995-A Part I, column (e)** — *"Check if patron"* (of an agricultural or horticultural
    /// cooperative).
    ///
    /// ★★★ This decides **which form is filed**, not just a box. Form 8995-A's own header: *"Use this
    /// form if your taxable income, before your qualified business income deduction, is above $191,950
    /// ($383,900 if married filing jointly), **or you're a patron of an agricultural or horticultural
    /// cooperative**."* So a patron **below** the threshold must file 8995-A too — and btctax would
    /// otherwise print the simplified Form 8995, which is the wrong form on a filed return.
    ///
    /// A `yes` REFUSES: it requires Schedule D (Form 8995-A), which computes the patron reduction that
    /// Part II line 14 subtracts, and btctax fills no such schedule. Refusing is the fail-closed
    /// direction; printing a return with line 14 blank would OVERSTATE the deduction.
    #[serde(default)]
    pub is_cooperative_patron: Option<bool>,
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
            // ★ `None` = never asked, for all three. A `Default` may not answer a question the filer
            //   was never put; the §199A(b)(2) amounts refuse where they are needed, and the SSTB
            //   declaration refuses unanswered like every other class-(A).
            qbi_w2_wages: None,
            qbi_ubia: None,
            is_sstb: None,
            is_cooperative_patron: None,
            expenses: Usd::ZERO,
            // ★ $0 = all business revenue arrived through the ledger, which is the common case and a
            //   real answer; the §G-22 out-of-scope-income declaration is what stops it being a
            //   silent one (see the field's doc).
            other_gross_receipts: Usd::ZERO,
            payments_requiring_1099: None,
            will_file_required_1099: None,
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
    /// **§163(h)(3)(B) — the ACQUISITION-DEBT CEILING declaration.** i1040sca (2024), *Limits on home
    /// mortgage interest*, transcribed rather than summarised because each limit is a separate test:
    ///
    /// - *"Your deduction for home mortgage interest is subject to a number of limits. If one or more of
    ///   the following limits applies, see Pub. 936 to figure your deduction."*
    /// - *"Limit on loans taken out on or before December 15, 2017. For qualifying debt taken out on or
    ///   before December 15, 2017, you can only deduct home mortgage interest on up to $1,000,000
    ///   ($500,000 if you are married filing separately) of that debt."*
    /// - *"Limit on loans taken out after December 15, 2017. For qualifying debt taken out after
    ///   December 15, 2017, you can only deduct home mortgage interest on up to $750,000 ($375,000 if
    ///   you are married filing separately) of that debt. If you also have qualifying debt subject to
    ///   the $1,000,000 limitation … the $750,000 limit for debt taken out after December 15, 2017, is
    ///   reduced by the amount of your qualifying debt subject to the $1,000,000 limit."*
    /// - *"Limit when loans exceed the fair market value of the home. If the total amount of all
    ///   mortgages is more than the fair market value of the home, see Pub. 936 to figure your
    ///   deduction."*
    ///
    /// A class-(A) DECLARATION, phrased so `true` — "I am inside every limit" — is the neutral answer
    /// that leaves line 8a at the full Form 1098 amount. `None` = never asked ⇒ refuse
    /// (`MortgageDebtLimitUnanswered`). `Some(false)` = "one of the limits bites" ⇒ ALSO refuse
    /// (`MortgageOverDebtLimit`): i1040sca's Line 8a instruction is *"Only enter on line 8a the
    /// deductible mortgage interest and points that were reported to you on Form 1098"*, a determinate
    /// NONZERO output of Pub. 936's Deductible Home Mortgage Interest Worksheet that btctax does not
    /// model. Deducting the full 1098 figure would UNDERSTATE the tax; printing $0 would OVERSTATE it by
    /// the whole deductible portion, and — unlike the mixed-use zero, which the line-8 checkbox
    /// discloses — Schedule A carries no box that could explain it.
    #[serde(default)]
    pub mortgage_within_debt_limit: Option<bool>,
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
    /// **Schedule A line 9 — "Investment interest. Attach Form 4952 if required. See instructions"**
    /// (§163(d)). The interest paid on money borrowed that is allocable to property held for
    /// investment.
    ///
    /// ★ It was NEVER COLLECTED, and the census recorded that honestly (`f1040sa.map.toml`, line 9,
    /// `rule = "unmodeled"`). The direction is safe — a forgone deduction only OVERSTATES tax — but a
    /// filer paying six figures of margin interest against a bitcoin position got no signal at all,
    /// and §163(d)(4)(B)(iii)'s election to treat net long-term capital gain as investment income is
    /// aimed at exactly that household.
    ///
    /// ★★ btctax builds **no Form 4952**, so this is only deductible in full under i4952's own
    /// exception — see [`ReturnInputs::filing_form_4952`], which carries the declaration and the
    /// bound. Anything else refuses.
    #[serde(default)]
    pub investment_interest: Usd,
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
    /// ★★ **Form 8995 line 3** — the prior-year qualified-business net **(loss)** carryforward, as a
    /// POSITIVE MAGNITUDE (the form pre-prints the parentheses). Line 4 combines it with line 2, so it
    /// REDUCES this year's QBI, and omitting it INFLATES the deduction and UNDERSTATES the tax.
    ///
    /// ★ It was left out on the reasoning that *"a Schedule C loss refuses upstream, so v1 never
    /// carries one"* — a non-sequitur: `ScheduleCLoss` is about the CURRENT year, while line 3 is a
    /// PRIOR-year figure the filer brings in from a return btctax did not compute. Exactly like
    /// [`Self::reit_ptp_carryforward_in`] eight lines up, which the same function has always consumed.
    ///
    /// ★★ **NEITHER ORACLE VALIDATES THIS** (`two-oracle-model` §G-9's limit): OTS takes it as a
    /// hand-fed input and Tax-Calculator has no channel for it at all, so oracle agreement here proves
    /// nothing. It is held by hand-computed KATs against i8995 lines 3/4/16 instead.
    #[serde(default)]
    pub qbi_carryforward_in: Usd,
    #[serde(default)]
    pub qbi_carryforward_in_provenance: CarryProvenance,
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
    /// §G-28/B4 — broker 1099-B TOTALS for Schedule D lines 1a/8a. Empty on a pure-crypto return.
    #[serde(default)]
    pub b_1099: Vec<Form1099B>,
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
    /// ★★ §G-20a — provenance for [`Self::capital_loss_carryforward_in`], as a SIBLING scalar.
    ///
    /// It is not inside `Carryforward` because that type lives in the **frozen** `tax/types.rs` (the
    /// crypto-delta engine), and a third `frozen_guard` pin exception for a field the delta path never
    /// reads would be a poor trade. Semantically it belongs here anyway: the provenance is a fact about
    /// this YEAR'S INPUT, not about the value type.
    ///
    /// **Why it exists at all:** without it a zero is uninterpretable — *"the filer has no carryover"*
    /// and *"nobody ever asked"* are the same bytes. `Computed` means btctax derived it from a prior
    /// year it actually computed; `User` (the default) means it is the filer's or nobody's.
    #[serde(default)]
    pub capital_loss_carryforward_in_provenance: CarryProvenance,
    /// ★★★ **Capital Loss Carryover Worksheet header** — *"If you and your spouse once filed a joint
    /// return and are filing separate returns for 2025, any capital loss carryover from the joint
    /// return can be deducted only on the return of the spouse who actually had the loss."*
    /// (`capital_loss_carryover::JOINT_RETURN_SOURCING`.)
    ///
    /// **Does any part of the carryover-in come from a JOINT return for a year you are now filing
    /// separately from, where the loss was your SPOUSE'S?** btctax stores one `Carryforward` per
    /// return and knows nothing about which spouse realised which loss, so it cannot perform the
    /// split the form requires: `Some(true)` REFUSES.
    ///
    /// A class-(A) DECLARATION, so `None` refuses too — and it is phrased so `false` is the neutral
    /// answer, per `widening-an-exemption-is-never-the-safe-edit`: the YES-condition is enumerated and
    /// every omission fails closed.
    ///
    /// ★ Collected rather than approximated because the form asks it and our input surface could not
    /// answer it. It became load-bearing when `--write-carryover` learned to roll §1212(b): before
    /// that, a mis-attributed carryover was the filer's own bad input; after it, btctax re-emits the
    /// figure as its OWN `Computed` value on next year's sworn Schedule D lines 6/14.
    #[serde(default)]
    pub carryover_includes_spouses_joint_loss: Option<bool>,
    /// ★★★ **Capital Loss Carryover Worksheet header** — *"If you excluded canceled debt from income
    /// in 2025, see Pub. 4681."* (`capital_loss_carryover::CANCELED_DEBT_EXCLUSION`; §108(b)(2)(G).)
    ///
    /// **Did you exclude cancelled or forgiven debt from income?** §108(b) then requires tax
    /// ATTRIBUTE REDUCTION, and §108(b)(2)(G) puts capital loss carryovers on that list — so the
    /// carryover surviving into next year is smaller than the worksheet alone says. btctax models no
    /// part of §108(b), so `Some(true)` REFUSES rather than carrying an unreduced figure.
    ///
    /// A class-(A) DECLARATION: `None` refuses, `false` is neutral.
    #[serde(default)]
    pub excluded_canceled_debt: Option<bool>,
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
    /// ★★ §G-20a — provenance for the charitable carryover **LIST AS A WHOLE**.
    ///
    /// `CharitableCarryItem` already carries a per-item provenance, which is useless for the case that
    /// matters: an **EMPTY vec has no items**, so an empty list carries no provenance at all — and an
    /// empty list is exactly the state that is ambiguous between "no carryover" and "never asked".
    #[serde(default)]
    pub charitable_carryover_in_provenance: CarryProvenance,
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
    /// **Schedule B line 7a's UNNUMBERED sub-question** — *"If 'Yes,' are you required to file FinCEN
    /// Form 114, Report of Foreign Bank and Financial Accounts (FBAR), to report that financial interest
    /// or signature authority?"*
    ///
    /// ★ A class-**(B)** SKIPPABLE, live only when [`Self::foreign_accounts`] is `Some(true)` — the
    /// form itself conditions it on 7a being "Yes". It was class (A) for two commits; `cbe651d`
    /// reversed that, because an unanswered sub-question is a lawful blank and refusing the whole
    /// return over it is the wrong instrument (the skip advisory names the FinCEN exposure instead).
    /// ★ PRE-MERGE M9 — this doc still said "class-(A) DECLARATION", which instructs a maintainer to
    /// restore exactly the refusal that reversal deliberately removed. Stays `Option` all the way to
    /// the PDF writer: `None`
    /// (7a was "No", so the question is not live) and `Some(false)` (it IS live and the filer answered
    /// "No") must print differently — an unwritten pair versus a checked "No" box.
    ///
    /// The form's own Caution is why this is not optional: *"If required, failure to file FinCEN Form
    /// 114 may result in substantial penalties."*
    #[serde(default)]
    pub fbar_filing_required: Option<bool>,
    /// 1040 header "you were a dual-status alien" — a class-(A) DECLARATION (P9 §2.5), live always. A single
    /// box asserting a fact whose unchecked state we print today from the MFS coupling alone; `None` ⇒ refuse
    /// (`DualStatusAlienUnanswered`), `Some(true)` ⇒ refuse unsupported (§63(c)(6)(B): NRA standard deduction
    /// is zero), `Some(false)` ⇒ proceed.
    /// ★★★ **Form 8283 Section B lines 5a / 5b / 5c** — the three restriction questions, asked ONCE
    /// for the whole return: *"did any of your donations have strings attached?"*
    ///
    /// The form asks three things about each donated property, and a **Yes** to any of them shrinks or
    /// kills the §170 deduction (Reg §1.170A-7 — a gift with retained rights is not a gift of the whole
    /// thing). btctax deducts at full fair market value, so a Yes means the number on the return is
    /// **WRONG**, and it refuses rather than compute it.
    ///
    /// ★★ **ONE return-level question, three per-donation boxes — sound, not a shortcut.** The filer
    /// states a UNIVERSAL ("none of my donations had any of these"), from which each box's answer
    /// follows for every donation. That is why the prompt ENUMERATES all three limbs in the form's own
    /// words: a "No" to something vaguer would be laundered into three specific answers the filer never
    /// gave. Enumerate the YES-conditions, default to refusing, so an omission fails closed.
    ///
    /// ★ It also dissolves §G-21's blocker. The registry is RETURN-shaped and these looked
    /// per-donation; asking the universal makes them return-shaped too, so no per-row machinery is
    /// needed. A **Yes** refuses rather than asking which donation — btctax cannot reduce the right
    /// gift's deduction, so the honest move is to send that year's 8283 to be completed by hand.
    #[serde(default)]
    pub donations_had_restrictions: Option<bool>,
    /// ★★★ **§170(f)(8) — the CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT**, asked as one return-level
    /// universal (the [`Self::donations_had_restrictions`] shape, and for the same structural reason).
    ///
    /// Schedule A lines 11 and 12 both print *"If you made any gift of $250 or more, see
    /// instructions"* on the form's face, and the first thing those instructions say is:
    ///
    /// > *"Gifts of $250 or more. You can deduct a gift of $250 or more only if you have a
    /// > contemporaneous written acknowledgment from the charitable organization showing the
    /// > information in (1) and (2) next."*
    /// >
    /// > *"In figuring whether a gift is $250 or more, don't combine separate donations."*
    /// >
    /// > *"To be contemporaneous, you must get the written acknowledgment from the charitable
    /// > organization by the date you file your return or the due date (including extensions) for
    /// > filing your return, whichever is earlier. Don't attach the contemporaneous written
    /// > acknowledgment to your return. Instead, keep it for your records."*
    ///
    /// §170(f)(8)(A) is a strict statutory precondition of ALLOWABILITY, not a recordkeeping nicety:
    /// *"No deduction shall be allowed … for any contribution of $250 or more unless the taxpayer
    /// substantiates the contribution by a contemporaneous written acknowledgment."*
    ///
    /// ★★ **FILING IS THE POINT OF NO RETURN**, which is what makes this a refusal rather than an
    /// advisory. §170(f)(8)(C) defines contemporaneous by the **earlier of** filing or the due date,
    /// so a filer who exports and files without a CWA has permanently extinguished the cure — a
    /// post-filing acknowledgment fails (C) by its terms (*Durden v. Comm'r*, T.C. Memo. 2012-140).
    /// The sharp line that keeps this from generalising: **refuse where filing itself extinguishes the
    /// cure; advise where the record can be assembled later.** §170(f)(17) bank-record substantiation
    /// for small cash gifts has no filing-linked deadline and is deliberately NOT gated.
    ///
    /// ★ Class (B) HERE and mandatory in [`crate::tax::return_1040::screen_absolute`], because
    /// liveness sees only `ReturnInputs`: whether the return itemizes is the computed §63(e) election,
    /// and the donations are in the LEDGER. `None` while live ⇒ refuse; `Some(false)` ⇒ refuse;
    /// `Some(true)` ⇒ proceed. A standard-deduction filer, or one whose every gift is under $250, is
    /// never asked — silence there forgoes nothing and asserts nothing.
    #[serde(default)]
    pub charitable_cwa_obtained: Option<bool>,
    /// ★★★ **Schedule D line 20 / Schedule A line 9 — ARE YOU FILING FORM 4952?**
    ///
    /// Schedule D line 20 asks, verbatim: *"Are lines 18 and 19 both zero or blank **and you are not
    /// filing Form 4952**? **Yes.** Complete the Qualified Dividends and Capital Gain Tax Worksheet
    /// … **No.** Complete the Schedule D Tax Worksheet."*
    ///
    /// ★★★ btctax used to check **Yes** UNCONDITIONALLY on the both-gains branch. The lines-18/19 half
    /// was sound — btctax refuses every return that could carry a §1250, §1202 or 28%-rate amount, and
    /// the form itself says *"both zero **or blank**"* — but the Form 4952 conjunct had **no source at
    /// all**: nothing on the return recorded it, and no question ever asked it. That is sworn
    /// testimony under §6065 that btctax invented, and it is the answered-ness invariant in its purest
    /// form. It reaches EVERY return whose Schedule D routes both-gains, including a $0-income
    /// household — not a rich-band item.
    ///
    /// ★ The tax was almost always right (a filer with no margin borrowing is indeed not filing Form
    /// 4952) — which is precisely why a value test could never find it. And when it is wrong it is
    /// wrong in the UNDERSTATING direction: the Schedule D Tax Worksheet the "No" branch leads to
    /// subtracts Form 4952 line 4g at its line 4.
    ///
    /// A class-(A) DECLARATION, `neutral: false` ("no, I am not filing one" is the answer that needs
    /// no form btctax lacks). `None` ⇒ refuse; `Some(true)` ⇒ refuse (btctax fills neither Form 4952
    /// nor the Schedule D Tax Worksheet); `Some(false)` ⇒ line 20 is checked **Yes** *because the
    /// filer said so*, and [`ScheduleAInputs::investment_interest`] may be deducted in full on line 9
    /// under i4952's exception — bounded by that exception's own first condition.
    #[serde(default)]
    pub filing_form_4952: Option<bool>,
    /// ★★★ **Form 8960 Part II line 9b — "State, local, and foreign income tax (see instructions)"**
    /// (§1411(c)(1)(B)); the state/local income tax the filer allocates to net investment income.
    ///
    /// i8960 (2024), *Line 9b—State, Local, and Foreign Income Tax*, transcribed rather than
    /// summarised because each sentence bounds a different thing:
    ///
    /// - *"Include state, local, and foreign income taxes you paid for the tax year that are
    ///   attributable to net investment income."*
    /// - *"Sales taxes aren't deductible in computing net investment income."*
    /// - *"You may not take a deduction for any foreign income taxes paid for the tax year if you
    ///   took a credit for any portion of them. See section 275(a)(4)."*
    /// - *"You can determine the portion of your state, local, and foreign income taxes allocable to
    ///   net investment income using any reasonable method."*
    ///
    /// ★★ **COLLECTED, never computed** (plan decision 6, `ADJUDICATION-2026-08-21.md` D5's build-shape
    /// guard 1). "Any reasonable method" is the FILER'S election — i8960 even says *"the reasonable
    /// method of allocation may differ from year to year"* — so btctax must not pick one for them.
    /// `None` = never answered ⇒ Form 8960 line 9b prints **BLANK**, and the whole Part II deduction is
    /// forgone, which can only OVERSTATE the tax (`Advisory::Form8960Line9bNotClaimed` says so, and
    /// names i8960's own worked example — the line 8 ÷ AGI ratio — as *a* reasonable method).
    /// `Some(x)` = the filer's own allocation, printed as entered.
    ///
    /// ★★★ **BOUNDED by §164(b)(6), and the bound is a VALIDATION, not a clamp.** §1411(c)(1)(B)
    /// reduces net investment income only by *"the deductions **allowed by this subtitle** which are
    /// properly allocable"*, and SALT above the §164(b)(6)(B) $10,000 / $5,000-MFS cap is not allowed
    /// by subtitle A at all — so there is nothing for §1411 to allocate. i8960's own allocation block
    /// agrees: the allocable item is *"State, local, and foreign income taxes **if properly deducted
    /// on your return** when calculating your U.S. regular income tax."* `screen_absolute` REFUSES a
    /// value above that bound rather than silently shrinking it, because a shrunk figure would be
    /// btctax choosing the allocation after all. See [`RefuseReason::Nii9bExceedsDeductedSalt`].
    ///
    /// ★ **The foreign component is structurally $0 here.** btctax's only foreign income tax is
    /// 1099-INT box 6 / 1099-DIV box 7, and it is taken UNCONDITIONALLY as the §904(j) foreign tax
    /// CREDIT (`assemble_absolute`) — §275(a)(4) then denies the deduction, which is the third
    /// sentence above. So the bound is the state/local income tax and nothing else.
    ///
    /// [`RefuseReason::Nii9bExceedsDeductedSalt`]: crate::tax::return_refuse::RefuseReason::Nii9bExceedsDeductedSalt
    #[serde(default)]
    pub form_8960_line9b: Option<Usd>,
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
    /// ★★★ **§G-22 / B11 — the SCOPE ATTESTATION.** Did the filer receive any income this tool never
    /// asked about? `None` = never asked ⇒ **refuses**. `Some(true)` ⇒ refuses (out of scope).
    /// `Some(false)` ⇒ the filer has affirmed there is none, and the return may be filed.
    ///
    /// **Why this exists, and why it is a DECLARATION and not a doc line.** btctax asks about HSA
    /// activity, dual-status alien status, foreign accounts and foreign trusts — and a "yes" to any of
    /// them refuses. It never asked about rental, royalty, farm, or K-1 income, so a filer with a
    /// rental could enter every other fact, get a clean packet with **no advisory and no refusal**, and
    /// file a return omitting §61 income. A filing trial reproduced exactly that with Publication 559's
    /// $8,183 of net rental income. `btctax limitations` names the boundary, but that is prose the
    /// filer must think to read, not a gate the return must pass.
    ///
    /// ★★ This is the [answered-ness invariant](super::questions) at the **scope boundary** rather than
    /// the money line, and the failure direction is an UNDERSTATEMENT. Before the question, the filer's
    /// silence *asserted* "I had no rental income" on a return signed under §6065; after it, silence
    /// merely *forgoes filing*. That is the doctrine's own sharp test — does the silence ASSERT or
    /// FORGO? — and it is what every human preparer asks.
    ///
    /// ★ ONE union question, not one per schedule. The out-of-scope set moves every tax year; the
    /// union is stable and the refusal message can enumerate. Scoped to **income only**: out-of-scope
    /// deductions and credits fail conservatively and are already advised, and widening this to every
    /// out-of-scope item is how it becomes the questionnaire nobody finishes.
    #[serde(default)]
    pub other_out_of_scope_income: Option<bool>,
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
    /// A **LOWER BOUND** on modified AGI that needs no answer to the §911/931/933 gate.
    ///
    /// ★★★ WHY A BOUND IS ENOUGH, AND WHY [`Self::modified_agi`] CANNOT SERVE HERE.
    ///
    /// `modified_agi` returns `None` when `has_income_exclusion` was never asked, and that is right
    /// for a consumer whose answer moves the wrong way under an unknown (the TY2025 SALT worksheet
    /// gives the filer the SMALLEST deduction, so it must not be handed an optimistic MAGI). But the
    /// gate is `live: |ri| ri.tax_year >= 2025`, so on **TY2024 — the only year btctax can file — it
    /// is never asked**, `modified_agi` is always `None`, and any consumer that refuses on `None`
    /// is dead code in production while its tests pass by setting the gate by hand.
    ///
    /// Some consumers are **monotonic**: the CTC phase-out only ever grows with MAGI, so a predicate
    /// like "the credit is entirely phased out" that holds at a LOWER BOUND holds at the true value
    /// too. Those need no answer — only a bound that is honest.
    ///
    /// ★★ The bound adds back only the NEGATIVE parts, which makes it valid without assuming the
    /// gate's answer OR the amounts' signs:
    ///   * gate `false` ⇒ MAGI = agi, and the bound ≤ agi because it adds only non-positive terms;
    ///   * gate `true`  ⇒ MAGI = agi + Σx, and the bound = agi + Σmin(x,0) ≤ agi + Σx.
    ///
    /// Both branches dominate the bound, so it is a lower bound under either answer. The four fields
    /// are exclusion AMOUNTS and negative values are nonsense, but nothing screens them
    /// (`first_negative_amount` explicitly waives them, "refused at the worksheet's point of need"),
    /// so the bound is written to survive one rather than to assume it away.
    ///
    /// ★ It is deliberately CONSERVATIVE where it matters: a filer carrying an unblessed $200,000
    /// exclusion gets a bound of `agi`, so a phase-out proof simply fails and they are told to check
    /// Schedule 8812. That is the r8 F2 protection — never talk a filer out of money they are owed —
    /// preserved by construction rather than by a separate guard.
    #[must_use]
    pub fn modified_agi_lower_bound(&self, agi: Usd) -> Usd {
        agi + [
            self.excluded_puerto_rico_income,
            self.form_2555_line45,
            self.form_2555_line50,
            self.form_4563_line15,
        ]
        .into_iter()
        .map(|x| x.min(Usd::ZERO))
        .sum::<Usd>()
    }

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
            // §G-22/B11 — `None` = never asked, and that REFUSES. A default may not answer it.
            other_out_of_scope_income: None,
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
            b_1099: Vec::new(),
            schedule_c: None,
            schedule_a: None,
            itemize_election: ItemizeElection::Auto,
            mfs_spouse_itemizes: None,
            sch1: Schedule1Inputs::default(),
            payments: Payments::default(),
            capital_loss_carryforward_in: Carryforward::default(),
            capital_loss_carryforward_in_provenance: CarryProvenance::default(),
            carryover_includes_spouses_joint_loss: None,
            excluded_canceled_debt: None,
            amt_carryover_same_as_regular: None,
            amt_depreciation_same_as_regular: None,
            charitable_carryover_in: Vec::new(),
            charitable_carryover_in_provenance: CarryProvenance::default(),
            qbi: QbiInputs::default(),
            foreign_accounts: None,
            foreign_trust: None,
            foreign_country_names: String::new(),
            fbar_filing_required: None,
            donations_had_restrictions: None,
            // §170(f)(8) — `None` = never asked. On a return that claims a §170 deduction with a
            // ≥$250 gift that REFUSES (`screen_absolute`), and a default may not answer it.
            charitable_cwa_obtained: None,
            // Schedule D line 20 / Schedule A line 9 — `None` = never asked, and that REFUSES.
            filing_form_4952: None,
            // ★ Form 8960 line 9b — `None` = the filer allocated nothing, so the line prints BLANK.
            //   A defaulted `Usd::ZERO` would swear on a signed return that the allocation IS zero.
            form_8960_line9b: None,
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
