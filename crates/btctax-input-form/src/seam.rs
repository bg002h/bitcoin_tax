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
    ReturnOptions,
    Taxpayer,
    Spouse,
    Address,
    Dependents,
    W2s,
    W2Box12,
    ScheduleA,
    ScheduleACharitable,
    Payments,
    /// ★ §G-22 — the QBI loss carryforwards. The ONLY carryforward family in the understatement
    /// direction, and the only one collectable through the form.
    Carryforwards,
    /// ★ §G-28/B1b — the §199A(b)(2) limitation amounts. NOT carryforwards: they are facts about the
    ///   business THIS year, and above the §199A(e)(2) threshold they cap the deduction.
    QbiLimitation,
    /// ★ spec 1099-DA T6 — the Form 1099-DA answers, one row per exchange PROVIDER the year's Form
    ///   8949 rows carry, two slots per row (covered / noncovered). Rows are SEEDED from the ledger's
    ///   keys by the renderer (the seam cannot see the ledger), never added by hand: an answer for a
    ///   key no row reads refuses as unread.
    BrokerReporting,
    // ── ★★★ R4 / T5 — one REPEATING section per supported information return, modelled on `W2s`. ──
    /// Form 1099-INT rows — `ri.int_1099`.
    Int1099s,
    /// Form 1099-DIV rows — `ri.div_1099`.
    Div1099s,
    /// Form 1099-B rows — `ri.b_1099` (the Schedule D line 1a/8a summary option).
    B1099s,
    /// Form 1099-G rows — `ri.g_1099`.
    G1099s,
    /// Form 1098-E rows — `ri.form_1098e` (Schedule 1 line 21).
    Form1098Es,
    /// ★★★ R5 / T5 — interest and dividends from the FILER'S OWN RECORDS, with no information
    /// return behind them: `ri.schedule_b_filer_records`. Its own section rather than more 1099
    /// rows, because the provenance is different by construction (`Source::FilerRecords`) and the
    /// section is live only when the filer says such income exists.
    ScheduleBFilerRecords,
    Declarations,
    /// ★★★ R3 / §5.1 — THE DOCUMENT CENSUS: one tri-state per document type. Its own section rather
    /// than more `Declarations` leaves, because the filer answers it FIRST — it is the question
    /// *"what is in the shoebox?"*, and every document section's liveness hangs off it.
    DocumentCensus,
    IncomeExclusions,
    Skippables,
}

/// Stable field identity. One per leaf across the v1 sections (spec §5.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldId {
    // ReturnOptions
    FilingStatus,
    ItemizeElection,
    // Taxpayer / Spouse (Person + header.ip_pin)
    TpFirstName,
    TpLastName,
    TpSsn,
    TpOccupation,
    TpPresidentialFund,
    IpPin,
    SpFirstName,
    SpLastName,
    SpSsn,
    SpOccupation,
    SpPresidentialFund,
    // Address
    AddrStreet,
    AddrCity,
    AddrState,
    AddrZip,
    // Dependents (per row)
    DepName,
    DepSsn,
    DepRelationship,
    DepDob,
    // W2 (per row)
    W2Owner,
    W2Employer,
    /// The employer's **EIN** (W-2 box b) — §6413(c)'s "more than one employer" test reads it.
    W2Ein,
    Box1Wages,
    Box2FedWh,
    Box3SsWages,
    Box4SsWh,
    Box5MedWages,
    Box6MedWh,
    Box7SsTips,
    Box17StateWh,
    Box19LocalTax,
    Box8AllocTips,
    Box10DepCare,
    /// ★★★ W-2 **box 13 *Statutory employee***. A checked box sends box 1 to Schedule C line 1, not
    /// to Form 1040 line 1a, so `true` refuses `StatutoryEmployeeW2`.
    W2Box13StatutoryEmployee,
    /// W-2 **box 14b *Treasury Tipped Occupation Code(s)*** (the 2026 revision) — the code Schedule
    /// 1-A Part II's Caution turns on.
    W2Box14bTtoc,
    // W2 box 12 (per row)
    Box12Code,
    Box12Amount,
    // Schedule A
    SaMedical,
    SaSaltRealEstate,
    SaSaltPersonalProp,
    SaSaltStateEst,
    SaSaltPriorYear,
    SaSaltSalesTaxAmt,
    SaMortgage1098,
    SaSaltUseSalesTax,
    SaMortgageAllUsed,
    /// §163(h)(3)(B) — the acquisition-debt-ceiling declaration (`FORM_QUESTIONS` index 13).
    SaMortgageWithinDebtLimit,
    /// Schedule A line 9 — investment interest (§163(d)).
    SaInvestmentInterest,
    /// ★ Form 8960 line 9b — the state/local income tax the filer allocates to net investment income
    /// (§1411(c)(1)(B)). It lives on `ReturnInputs`, not `ScheduleAInputs`, because it is a Form 8960
    /// line; it is RENDERED here because every number that bounds it — line 5a, line 5e, the
    /// §164(b)(5) election — is on the Schedule A immediately above it.
    Nii8960Line9b,
    // Schedule A charitable (per row)
    CharClass,
    CharAmount,
    // Payments
    PayEstimated,
    PayExtension,
    PayOtherWh,
    // Declarations (from FORM_QUESTIONS) + the 7b country text
    DeclDependentTaxpayer,
    DeclDependentSpouse,
    DeclMfsSpouseItemizes,
    DeclForeignAccounts,
    DeclForeignTrust,
    DeclHsaActivity,
    DeclDualStatusAlien,
    /// Form 6251 line 3 — is the mortgaged dwelling AMT-qualified? (i6251 p.8.)
    DeclAmtQualifiedDwelling,
    DeclHasIncomeExclusion,
    /// §G-22/B11 — the scope attestation: income this tool never asked about.
    DeclOtherOutOfScopeIncome,
    /// Schedule D line 20 / Schedule A line 9 — is the filer filing Form 4952? (`FORM_QUESTIONS` 14.)
    DeclFilingForm4952,
    /// §G-28/B1b — Form 8995-A Part I column (b): is the business a specified service trade or business?
    ScheduleCIsSstb,
    /// §G-28/B1b — Form 8995-A Part I column (e): is the filer a patron of an agricultural or
    /// horticultural cooperative? Decides WHICH §199A form is filed, at any income.
    ScheduleCIsCooperativePatron,
    /// §G-28/B1b — Form 8995-A line 4.
    QbiW2Wages,
    /// §G-28/B1b — Form 8995-A line 7.
    QbiUbia,
    ExclPuertoRico,
    Excl2555L45,
    Excl2555L50,
    Excl4563L15,
    /// Form 6251 line 2k — does the AMT capital-loss carryover equal the regular one?
    DeclAmtCarryoverSame,
    /// Form 6251 line 2l — is the depreciation inside the Schedule C expense total the same for the AMT?
    DeclAmtDepreciationSame,
    /// Capital Loss Carryover Worksheet header (§1212(b)) — does the carryover-in include a loss that
    /// was the SPOUSE'S, from a joint year now filed separately? (`FORM_QUESTIONS` 15.)
    DeclCarryoverIncludesSpousesJointLoss,
    /// Capital Loss Carryover Worksheet header (§108(b)(2)(G)) — was cancelled debt excluded from
    /// income, requiring tax-attribute reduction? (`FORM_QUESTIONS` 16.)
    DeclExcludedCanceledDebt,
    ForeignCountryNames,
    // Skippables (from SKIPPABLE_QUESTIONS); SALT election = SaSaltUseSalesTax in Schedule A above
    BlindTaxpayer,
    BlindSpouse,
    DobTaxpayer,
    DobSpouse,
    DodTaxpayer,
    DodSpouse,
    /// Schedule B line 7a's unnumbered FBAR sub-question (FinCEN Form 114) — class (B): nothing on the
    /// return reads it, so silence is lawful.
    FbarFilingRequired,
    /// §G-9 death gates — class (B): silence FORGOES the §63(f) age-65 addition, never grants it.
    TaxpayerDiedDuringYear,
    SpouseDiedDuringYear,
    /// Schedule C lines I / J — the Form-1099 compliance pair. Class (B): no figure reads them and the
    /// form prints no Caution, but §6721/§6722 exposure is real, so the skip advises.
    ScheduleC1099Required,
    ScheduleC1099Filed,
    /// Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21).
    DonationsHadRestrictions,
    /// §170(f)(8) — the contemporaneous-written-acknowledgment universal (`SKIPPABLE_QUESTIONS` 15).
    CharitableCwaObtained,
    /// FR-29 — Form 8615 condition 3, the age + earned-income-support test (`SKIPPABLE_QUESTIONS` 16).
    Form8615Condition3AgeSupport,
    /// FR-29 — Form 8615 condition 4, "at least one of your parents was alive" (`SKIPPABLE_QUESTIONS`
    /// 17). The one `FieldKind::Enum` skippable: "CannotKnow" is a third ANSWER, not a blank.
    Form8615Condition4ParentAlive,
    /// FR-29 — the SPEC §6.3 dead-end fact (`SKIPPABLE_QUESTIONS` 18). ★ Its registry accessors
    /// INVERT: the label asks what the filer CAN supply, the leaf records what they cannot.
    Form8615ParentIdentityUnobtainable,
    // Carryforwards (§G-22) — Form 8995 lines 7 and 3. Both are prior-year LOSSES that REDUCE a
    // deduction, so omitting either UNDERSTATES tax. They were import-only until 2026-07-31.
    QbiReitPtpCarryforwardIn,
    QbiCarryforwardIn,
    // ★ spec 1099-DA T6 — the two slots of a `BrokerReporting` row (`broker_reporting.<provider>`).
    /// The answer for the provider's COVERED lots (bought on the venue on/after 2026-01-01).
    BrokerCovered,
    /// The answer for the provider's NONCOVERED lots (everything else the venue sold for the filer).
    BrokerNoncovered,
    // ── ★★★ R3 / §5.1 — the eighteen DOCUMENT CENSUS rows. One `FieldId` per document TYPE.
    //    Each delegates to its `FORM_QUESTIONS` entry, so the prompt, the liveness and the refusal
    //    are the registry's and are written exactly once.
    /// Census: did the filer receive one or more Form W-2?
    DocW2,
    /// Census: did the filer receive one or more Form 1099-INT?
    DocInt1099,
    /// Census: did the filer receive one or more Form 1099-DIV?
    DocDiv1099,
    /// Census: did the filer receive one or more Form 1099-B?
    DocB1099,
    /// Census: did the filer receive one or more Form 1099-G?
    DocG1099,
    /// Census: did the filer receive one or more Form 1098?
    DocForm1098,
    /// Census: did the filer receive one or more Form 1098-E?
    DocForm1098e,
    /// Census: did the filer receive one or more Form 1099-R?
    DocR1099,
    /// Census: did the filer receive one or more Form SSA-1099 / RRB-1099?
    DocSsa1099,
    /// Census: did the filer receive one or more Form 1099-NEC / 1099-MISC / 1099-K?
    DocNecMiscK1099,
    /// Census: did the filer receive one or more Schedule K-1?
    DocK1,
    /// Census: did the filer receive one or more rental real estate / royalties (Schedule E)?
    DocScheduleERental,
    /// Census: did the filer receive one or more Form 1099-S?
    DocS1099,
    /// Census: did the filer receive one or more Form 1099-OID?
    DocOid1099,
    /// Census: did the filer receive one or more Form W-2G?
    DocW2g,
    /// Census: did the filer receive one or more Form 1099-C?
    DocC1099,
    /// Census: did the filer receive one or more Form 1095-A?
    DocA1095,
    /// Census: did the filer receive one or more Form 1098-T?
    DocT1098,
    /// ★ R10.4 / T4b — is the filing status CARRIED by the year-N+1 opener still this year's status?
    ///   Live only on a year the opener made (`opened_from.is_some()`).
    DeclFilingStatusConfirmed,
    // ── ★★★ R3 / T5 — the DOCUMENT-LESS INCOME DOOR's four declarations. ────────────────────────
    /// Wages from an employer who issued no Form W-2 (live iff `documents.w2 == Some(false)`).
    DeclWagesWithoutW2,
    /// Interest or dividends with no Form 1099-INT / 1099-DIV behind them.
    DeclInterestOrDividendsWithout1099,
    /// A state or local income tax refund with no Form 1099-G.
    DeclStateRefundWithout1099g,
    /// §111(a) — did the PRIOR-YEAR return itemize? Return-level (R3/I1).
    DeclItemizedPriorYear,
    // ── ★★★ R4 / T5 — Form 1099-INT (per row). One Field per COLLECTED box, named for the box. ──
    /// The payer as printed on the form.
    Int1099Payer,
    /// R10.2 — the payer's TIN, the cross-year document identity.
    Int1099PayerTin,
    /// R10.2 — the date this row was transcribed off the paper.
    Int1099TranscribedOn,
    /// Box 1 — *Interest income* → Schedule B line 1 → 1040 line 2b.
    Int1099Box1Interest,
    /// Box 2 — *Early withdrawal penalty* → Schedule 1 line 18.
    Int1099Box2EarlyWithdrawal,
    /// Box 3 — *Interest on U.S. Savings Bonds and Treasury obligations* → 1040 line 2b.
    Int1099Box3Treasury,
    /// Box 4 — *Federal income tax withheld* → 1040 line 25b.
    Int1099Box4FedWithheld,
    /// Box 6 — *Foreign tax paid* → the §904(j) election on Schedule 3 line 1.
    Int1099Box6ForeignTax,
    /// Box 8 — *Tax-exempt interest* → 1040 line 2a.
    Int1099Box8TaxExempt,
    /// Box 9 — *Specified private activity bond interest* — a refuse-guard (AMT preference).
    Int1099Box9PrivateActivity,
    /// Box 10 — *Market discount* → Schedule B line 1 and the 1040 line 2b sum.
    Int1099Box10MarketDiscount,
    /// Box 11 — *Bond premium* — a refuse-guard (§171, Pub. 550).
    Int1099Box11BondPremium,
    /// Box 12 — *Bond premium on Treasury obligations* — a refuse-guard.
    Int1099Box12BondPremiumTreasury,
    /// Box 13 — *Bond premium on tax-exempt bond* — a refuse-guard.
    Int1099Box13BondPremiumTaxExempt,
    // ── ★★★ R4 / T5 — Form 1099-DIV (per row). ──────────────────────────────────────────────────
    Div1099Payer,
    Div1099PayerTin,
    Div1099TranscribedOn,
    /// Box 1a — *Total ordinary dividends* → 1040 line 3b (it INCLUDES box 1b).
    Div1099Box1aOrdinary,
    /// Box 1b — *Qualified dividends* → 1040 line 3a.
    Div1099Box1bQualified,
    /// Box 2a — *Total capital gain distr.* → Schedule D line 13.
    Div1099Box2aCapGain,
    /// Box 2b — *Unrecap. Sec. 1250 gain* — a refuse-guard.
    Div1099Box2bUnrecap1250,
    /// Box 2c — *Section 1202 gain* — a refuse-guard.
    Div1099Box2cSection1202,
    /// Box 2d — *Collectibles (28%) gain* — a refuse-guard.
    Div1099Box2dCollectibles,
    /// Box 4 — *Federal income tax withheld* → 1040 line 25b.
    Div1099Box4FedWithheld,
    /// Box 5 — *Section 199A dividends* → the QBI deduction.
    Div1099Box5Section199a,
    /// Box 7 — *Foreign tax paid* → the §904(j) election.
    Div1099Box7ForeignTax,
    /// Box 12 — *Exempt-interest dividends* → 1040 line 2a.
    Div1099Box12ExemptInterest,
    /// Box 13 — *Specified private activity bond interest dividends* — a refuse-guard.
    Div1099Box13PrivateActivity,
    // ── ★★★ R4 / T5 — Form 1099-B (per row), the Schedule D line 1a/8a SUMMARY option. ──────────
    B1099Payer,
    B1099PayerTin,
    B1099TranscribedOn,
    /// Schedule D line 1a(d) — short-term proceeds (1099-B box 1d, short-term).
    B1099ShortTermProceeds,
    /// Schedule D line 1a(e) — short-term cost or other basis (box 1e, short-term).
    B1099ShortTermBasis,
    /// Schedule D line 8a(d) — long-term proceeds (box 1d, long-term).
    B1099LongTermProceeds,
    /// Schedule D line 8a(e) — long-term cost or other basis (box 1e, long-term).
    B1099LongTermBasis,
    /// ★★★ THE GATE — box 12 checked AND no adjustments, both limbs named in the prompt.
    B1099BasisReportedNoAdjustments,
    // ── ★★★ R4 / T5 — Form 1099-G (per row). ────────────────────────────────────────────────────
    G1099Payer,
    G1099PayerTin,
    G1099TranscribedOn,
    /// Box 1 — *Unemployment compensation* → Schedule 1 line 7.
    G1099Box1Unemployment,
    /// Box 2 — *State or local income tax refunds, credits, or offsets* → Schedule 1 line 1,
    /// through the return-level prior-year-itemized gate.
    G1099Box2StateRefund,
    /// Box 4 — *Federal income tax withheld* → 1040 line 25b.
    G1099Box4FedWithheld,
    /// Box 10 — *Family leave benefits* (Rev. December 2026) — a refuse-guard.
    G1099Box10FamilyLeave,
    // ── ★★★ R4 / T5 — Form 1098-E (per row). ────────────────────────────────────────────────────
    Form1098eLender,
    Form1098eLenderTin,
    Form1098eTranscribedOn,
    /// Box 1 — *Student loan interest received by lender* → Schedule 1 line 21 (§221).
    Form1098eBox1Interest,
    // ── ★★★ R5 / T5 — the filer's-records rows for Schedule B lines 1 / 5. ──────────────────────
    SbRecordPayerName,
    /// The buyer's SSN on a seller-financed mortgage (`i1040sb--2025.txt:24`).
    SbRecordPayerSsn,
    /// The buyer's address on a seller-financed mortgage (`i1040sb--2025.txt:77`).
    SbRecordPayerAddress,
    SbRecordAmount,
    /// Which Schedule B list the row joins — line 1 (interest) or line 5 (dividend).
    SbRecordKind,
}

/// The value shape of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Money,
    Text,
    Bool,
    TriState,
    Date,
    Enum(&'static [&'static str]),
    Secret,
}

/// A field value crossing the seam (spec §4/§5.7). Owned (serde), so it is the web wire.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Money(Usd),
    Text(String),
    Bool(bool),
    TriState(Option<bool>),
    Date(Option<Date>),
    Choice(String),      // an Enum choice by its stable name
    Secret(SecretView),  // OUTBOUND only (get) — presence, never digits
    SecretEntry(String), // INBOUND only (set) — masked Debug; get never returns it
}

/// A secret's presence, never its digits (spec §4/§5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretView {
    Empty,
    Set { masked: String },
}

impl SecretView {
    /// ★ The single guarded producer of a `Set` view (spec §4/§5.5; folds review follow-up (b)). The two
    /// maskers in `spec::sections` are its ONLY callers, and this guard rejects a `masked` string that still
    /// carries a run of 5+ raw digits (a raw SSN is 9 digits, an IP PIN 6) — so a raw secret can never be
    /// stored as a "mask", even by a future caller. The SSN last-4 reveal (`***-**-6789`) has only a 4-digit
    /// run and passes.
    pub(crate) fn set_masked(masked: String) -> SecretView {
        debug_assert!(
            !masked.as_bytes().windows(5).any(|w| w.iter().all(u8::is_ascii_digit)),
            "SecretView::set_masked was given a string with a 5+ digit run (a raw secret?): {masked:?}"
        );
        SecretView::Set { masked }
    }
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::SecretEntry(_) => write!(f, "SecretEntry(***)"), // never leak digits
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

/// The un-answer closure a delegating `Field` carries (review I-1). Aliased so the `Option`-wrapped field
/// stays within clippy's type-complexity budget.
pub type ClearFn = fn(&mut ReturnInputs, &RowAddr) -> Result<(), SetError>;

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
    /// ★ The un-answer path (spec §5.7 M-6, review I-1). `Some` only for fields whose clear must write a
    /// specific empty the plain per-kind `set(empty)` path cannot express — the 13 registry-delegating
    /// tri-state/date leaves, which clear their underlying `Option` leaf to `None` (a definite-only registry
    /// `set` cannot). `None` for every plain field; `apply` then clears it via `set(empty_for_kind)`.
    pub clear: Option<ClearFn>,
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
        // ★ `add`/`remove` REPORT (review I-4): an absent parent (`[w2_i]` with no such W-2, or a nested
        // group whose owning optional-singleton is `None`) or an out-of-range row → `Err(NoSuchRow)`, never a
        // silent no-op that lies `Ok` on the wire. `apply` propagates the `Result`.
        add: fn(&mut ReturnInputs, &RowAddr) -> Result<(), SetError>,
        remove: fn(&mut ReturnInputs, &RowAddr) -> Result<(), SetError>,
    },
}

/// An edit from a renderer (spec §5.7). Serde-serializable — the web wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    SetField {
        id: FieldId,
        addr: RowAddr,
        value: FieldValue,
    },
    ClearField {
        id: FieldId,
        addr: RowAddr,
    },
    AddRow {
        section: SectionId,
        parent: RowAddr,
    },
    RemoveRow {
        section: SectionId,
        addr: RowAddr,
    },
    CreateSection {
        section: SectionId,
    },
    DeleteSection {
        section: SectionId,
    },
}

/// Where a `RefuseReason` points in the form (spec §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    Field(FieldId),
    Section(SectionId),
    NotInForm { note: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError {
    WrongKind,
    NoSuchRow,
    Immutable,
    /// ★★★ R3 — **a census row answered `No` while rows of that document are transcribed.**
    ///
    /// The `DeleteSection(ScheduleA)` I-10 precedent: the write is refused, the rows are NOT silently
    /// deleted, and the renderer offers *remove N rows and answer No* with a payload-confirm. A
    /// silent delete here would destroy transcribed testimony on one keystroke, and a silent accept
    /// would leave the return in the `DocumentCensusContradicted` state — a "no" the data
    /// contradicts — which commit then refuses with no in-form remedy.
    ContradictsTranscribedRows {
        /// How many rows of that document the return carries.
        rows: usize,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    NotANumber,
    Negative,
    BadDate,
    BadSsn,
    BadIpPin,
    NotAChoice,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    NotChosenYet,
    WrongFirstEdit,
    SetError(SetError),
    NoSuchSection,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Edit/FieldValue seam serializes losslessly (the web wire, spec §4/M-5). Task-2(d): exercise
    /// EVERY `FieldValue` kind (not just Money), across `SetField` + `ClearField` — a kind whose serde broke
    /// would otherwise slip through. (`SecretEntry` masks only its Debug, not its serde: the inbound digits
    /// do go over the wire by design; only get-side `Secret` is presence-only.) The four structural `Edit`
    /// variants (AddRow/RemoveRow/CreateSection/DeleteSection) are trivial derives, not re-exercised here.
    #[test]
    fn edit_roundtrips_through_json() {
        use rust_decimal_macros::dec;
        use time::macros::date;
        let values = [
            FieldValue::Money(dec!(50000)),
            FieldValue::Text("123 Main St, Anytown".into()),
            FieldValue::Bool(true),
            FieldValue::Bool(false),
            FieldValue::TriState(Some(true)),
            FieldValue::TriState(None),
            FieldValue::Date(Some(date!(2025 - 01 - 02))),
            FieldValue::Date(None),
            FieldValue::Choice("Single".into()),
            FieldValue::Secret(SecretView::Empty),
            FieldValue::Secret(SecretView::Set {
                masked: "***-**-6789".into(),
            }),
            FieldValue::SecretEntry("123456789".into()),
        ];
        for value in values {
            let e = Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: value.clone(),
            };
            let j = serde_json::to_string(&e).unwrap();
            let back: Edit = serde_json::from_str(&j).unwrap();
            assert_eq!(e, back, "SetField round-trip lost data for {value:?}");
        }
        // The other Edit variant + a multi-index addr also round-trip losslessly.
        let clear = Edit::ClearField {
            id: FieldId::Box1Wages,
            addr: RowAddr(vec![2, 1]),
        };
        let j = serde_json::to_string(&clear).unwrap();
        assert_eq!(clear, serde_json::from_str::<Edit>(&j).unwrap());
    }

    /// A SecretView never carries digits; SecretEntry is inbound-only and masks its Debug.
    #[test]
    fn secret_view_is_presence_only_and_entry_masks_debug() {
        assert_eq!(
            SecretView::Set {
                masked: "***-**-6789".into()
            },
            SecretView::Set {
                masked: "***-**-6789".into()
            }
        );
        let entry = FieldValue::SecretEntry("123456789".into());
        assert!(
            !format!("{entry:?}").contains("123456789"),
            "SecretEntry Debug must not leak digits"
        );
    }
}
