//! Full-return v1 **fail-closed refuse-guard** (Phase 1 task 4 / SPEC §4.10 / §3.4).
//!
//! A wrong full return is worse than a refusal. Any captured-but-unmodeled input that could make the
//! return *wrong* (understate tax, misstate a figure, or require a mandatory attachment v1 can't produce)
//! yields a [`Refusal`] — never a silent value. This module screens the **input-screenable** rows (those
//! decidable from `ReturnInputs` + the year tables). The **compute-dependent** rows — Schedule C net < 0,
//! Form 8615 kiddie tax (unearned income > threshold) — and the **ledger-dependent** rows — ≥2 SE
//! earners, business-flagged crypto interest, §1250/§1202/28% crypto — are screened in Phase 2/3 where
//! the assembled income / ledger is available.
//!
//! ★ This list USED TO name "taxable income ≤ 0 with a carryforward" among the compute-dependent rows.
//! Widening (A) lifted that refusal and deleted its variant; such a year now files.
//!
//! Uses a NEW domain type (not the ledger's shared `state::BlockerKind`, which is exhaustively matched
//! across the reconcile system) — additive, per SPEC §2. A `Refusal` maps to
//! `TaxOutcome::NotComputable(..)` at the report boundary (Phase 4).
use crate::conventions::Usd;
use crate::tax::return_inputs::{
    Box12Entry, CharitableCarryItem, CharitableClass, CharitableGift, Form1099Div, Form1099G,
    Form1099Int, Owner, Payments, QbiInputs, ReturnInputs, Schedule1Inputs, Schedule1aInputs,
    ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::tables::{FullReturnParams, TaxTable, EMPLOYEE_OASDI_RATE};
use crate::tax::types::{Carryforward, FilingStatus};
use rust_decimal_macros::dec;

/// The W-2 box-12 codes that are inert for a Common W-2 household return (elective deferrals + purely
/// informational). Any OTHER code refuses (SPEC §4.10 / audit I1 — an allowlist, not a blocklist).
const INERT_BOX12_CODES: &[&str] = &["D", "E", "F", "G", "H", "S", "AA", "BB", "EE", "DD"];

/// The §402(g) elective-deferral codes whose cross-employer sum is capped (SPEC F3).
const ELECTIVE_DEFERRAL_CODES: &[&str] = &["D", "E", "F", "G", "S"];

/// Why a full return is refused (fail-closed). One variant per SPEC §4.10 input-screenable row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    /// A captured money amount is negative. Every full-return input is a form-box MAGNITUDE (≥ 0); signs
    /// are produced by the computation, never the input. A negative value is a corrupt import that could
    /// otherwise *offset* an accumulated refusal threshold (e.g. §402(g), §904(j)) into passing (R2-I1).
    NegativeAmount(String),
    /// ★★★ Form 8283 Section B lines 5a/5b/5c — the restriction questions — are unresolved: either
    /// unanswered, or answered **Yes**. A Yes means at least one donated property carried a restriction
    /// or a retained right, which REDUCES or DENIES the §170 deduction (Reg §1.170A-7). btctax deducts
    /// at full FMV and cannot tell WHICH gift, so it refuses rather than file a number it knows is too
    /// large. Raised only for a year that actually files a **Section B** 8283 — see
    /// `screen_absolute`, which has the ledger AND the computed itemize election.
    DonationRestrictionsUnresolved,
    /// ★ spec 1099-DA R1 — a live year (basis regime AND ≥ 1 exchange disposition) has rows under a
    /// (provider, cohort) key the filer has not answered. The tool never chooses a Form 8949 box for a
    /// broker-reported row on its own (port report §6 rule 18).
    BrokerReportingUnanswered {
        provider: String,
        cohort: crate::forms::Cohort,
        year: i32,
    },
    /// spec 1099-DA R2 — the filer answered `Mixed`: the 1099-DAs for these rows disagree among
    /// themselves, so no single box is true of the set. The per-lot import is the exit.
    BrokerReportingMixed {
        provider: String,
        cohort: crate::forms::Cohort,
    },
    /// spec 1099-DA R2 — the filer answered `BasisDiffers`: the row needs the broker's basis in
    /// column (e) and the correction in (g), which only a per-lot 1099-DA import can supply.
    BrokerBasisDiffers {
        provider: String,
        cohort: crate::forms::Cohort,
    },
    /// spec 1099-DA R1 — an answer the tool would never read: for a key with NO rows this year, or on
    /// a year where the question is not live (no basis regime, or no exchange disposition). A
    /// declaration about nothing is fabricated; one silently discarded is testimony overridden by a
    /// default — both refuse.
    BrokerAnswerUnread {
        provider: String,
        cohort: crate::forms::Cohort,
        year: i32,
    },
    /// ★★★ **§170(f)(8)** — the contemporaneous-written-acknowledgment question is unresolved on a
    /// return that CLAIMS a charitable deduction and has at least one single contribution of $250 or
    /// more: either unanswered, or answered **No**.
    ///
    /// §170(f)(8)(A): *"No deduction shall be allowed under subsection (a) for any contribution of
    /// $250 or more unless the taxpayer substantiates the contribution by a contemporaneous written
    /// acknowledgment."* It is a precondition of allowability, so proceeding on silence claims a
    /// deduction the statute may deny — an understatement — while §170(f)(8)(C)'s *"earlier of"* rule
    /// means **filing itself extinguishes the cure**. Both halves are required for a refusal; a
    /// substantiation rule with no filing-linked deadline (§170(f)(17) bank records) gets an advisory,
    /// not a gate.
    ///
    /// Raised only by `screen_absolute`, which has the ledger AND the computed §63(e) election — the
    /// `DonationRestrictionsUnresolved` pattern exactly.
    CharitableCwaUnresolved,
    /// A **non-crypto NONCASH** charitable gift whose total exceeds the $500 Form 8283 threshold. Those
    /// amounts reach Schedule A line 12, but btctax holds no property details for them (no description,
    /// no acquisition date, no appraiser), so it can produce no 8283 rows — the packet would attach a
    /// Form 8283 that UNDER-REPORTS its own property list. An incomplete required attachment is a
    /// §170(f)(11) denial risk, and §3.4's conservative-omission carve-out does not apply: the omission
    /// is not taxpayer-favorable, it jeopardizes a deduction the filer is claiming (ARCH-P6.3a Q6).
    NonCryptoNoncashGift,
    /// A `Owner::Spouse`-tagged item (W-2 / Schedule C) on a non-joint return — no spouse's income is on
    /// a Single/HoH/MFS/QSS return, and trusting the tag would split one person's per-owner limits into
    /// two buckets, evading the §402(g) cap (R2-I2).
    SpouseOwnerWithoutJointReturn,
    /// `Some(true)` foreign trust → Form 3520 (out of scope, R2-I3).
    ForeignTrust,
    /// Schedule B files but Part III line 7a (foreign accounts) or 8 (foreign trust) is unanswered
    /// (`None`) — fail-loud rather than guess a disclosure answer (SPEC §7.1 / I7 / P2-I1).
    ScheduleBPart3Unanswered,
    /// A Schedule A `salt_sales_tax_amount` is set but the §164(b)(5) sales-tax election is OFF — a silent
    /// drop of the amount would hide an input error, so fail loud (SPEC §4.6 / R3-M9).
    SaltSalesTaxWithoutElection,
    /// MFS return without `mfs_spouse_itemizes` answered — §63(c)(6) couples the spouses' std/itemize
    /// choice, so it's required (`None` ⇒ fail-loud, G15).
    MfsSpouseItemizeUnknown,
    /// "Someone can claim you as a dependent" is UNANSWERED. Required on EVERY return: it selects the
    /// §63(c)(5) dependent standard-deduction floor over the basic std, gates the §1(g)/Form-8615
    /// kiddie-tax refusal, and prints a checkbox on the 1040 itself. Guessing `false` UNDERSTATES tax and
    /// files a false checkbox; guessing `true` overstates. Fail loud (D-8).
    DependentStatusUnanswered,
    /// "Someone can claim your spouse as a dependent" is unanswered on a return that HAS a spouse. Same
    /// reasoning as the taxpayer flag; only asked when a spouse is on the return (D-8).
    DependentSpouseStatusUnanswered,
    /// P9 §2.4 — the §223 HSA-activity DECLARATION is `None` (never asked). Live on EVERY return, because
    /// the answer is what decides whether Form 8889 files (so it cannot be scoped by whether 8889 files).
    /// An unasked distribution omits gross income + a 20% additional tax (§223(f)) — fail loud.
    HsaActivityUnanswered,
    /// P9 §2.5 — the dual-status-alien DECLARATION (1040 header) is `None`. Live always: a single box whose
    /// unchecked state we print today, and §63(c)(6)(B) zeroes an NRA's standard deduction. Fail loud.
    DualStatusAlienUnanswered,
    /// P9 §2.7 — the §163(h)(3)(F) mixed-use-mortgage DECLARATION is `None`, on a Schedule A carrying
    /// mortgage interest. Fail loud rather than print line 8a with the box in an unaffirmed state.
    MixedUseMortgageUnanswered,
    /// **§163(h)(3)(B)** — the acquisition-debt-ceiling DECLARATION is `None`, on a Schedule A carrying
    /// mortgage interest. i1040sca's *Limits on home mortgage interest* block states four limits and
    /// btctax models only the mixed-use one; without the answer it would deduct 100% of the Form 1098
    /// amount for a filer the statute caps, which UNDERSTATES the tax on a **filed** figure that neither
    /// oracle can catch (both consume line 8a as an input — §G-9). Fail loud.
    MortgageDebtLimitUnanswered,
    /// **§163(h)(3)(B)** — answered ADVERSELY ("one of the debt limits bites"). See
    /// [`ScheduleAInputs::mortgage_within_debt_limit`] for why neither available number is filable.
    ///
    /// [`ScheduleAInputs::mortgage_within_debt_limit`]: crate::tax::return_inputs::ScheduleAInputs::mortgage_within_debt_limit
    MortgageOverDebtLimit,
    /// **Schedule D line 20 / Schedule A line 9** — the Form 4952 declaration is unanswered. Line 20
    /// asserts *"you are not filing Form 4952"* on every both-gains return, and nothing on the return
    /// recorded it: btctax was signing that clause for the filer under §6065.
    Form4952DeclarationUnanswered,
    /// **Schedule D line 20 / Schedule A line 9** — the filer IS filing Form 4952 (or their line-9
    /// amount exceeds i4952's no-Form-4952 exception, which requires one). Line 20 is then **No**,
    /// which routes to the **Schedule D Tax Worksheet** — and btctax fills neither that worksheet nor
    /// Form 4952. The SDTW subtracts Form 4952 line 4g at its line 4, so answering Yes anyway
    /// UNDERSTATES the tax.
    Form4952Required,
    /// ★★★ **Form 8960 line 9b** — the collected state/local income tax allocated to net investment
    /// income EXCEEDS what §164(b)(6) let the filer deduct.
    ///
    /// §1411(c)(1)(B) reduces net investment income only by *"the deductions **allowed by this
    /// subtitle** which are properly allocable"*, and §164(b)(6)(B) caps the SALT *"taken into
    /// account"* at $10,000 ($5,000 MFS) — so a dollar of state income tax above the cap is not
    /// allowed by subtitle A at all and there is nothing for §1411 to allocate. i8960's own allocation
    /// block says the same: the allocable item is SALT *"if **properly deducted on your return** when
    /// calculating your U.S. regular income tax."*
    ///
    /// Three ways to exceed the bound, one refusal (`ADJUDICATION-2026-08-21.md` D5):
    ///
    ///   (a) the amount is simply larger than min(line 5a, line 5e, the cap);
    ///   (b) the filer took the **standard deduction**, so no state income tax was deducted at all
    ///       and the bound is $0;
    ///   (c) the filer made the §164(b)(5) **general-sales-tax election**, so Schedule A line 5a is
    ///       sales tax — and i8960 is express: *"Sales taxes aren't deductible in computing net
    ///       investment income."* The bound is $0 on that branch even for a large line 5e.
    ///
    /// ★ It REFUSES rather than clamping. A clamp would be btctax choosing the filer's allocation —
    /// the one thing D5's build-shape guard forbids — and it would silently rewrite a figure signed
    /// under 26 USC 6065.
    Nii9bExceedsDeductedSalt,
    /// ★★★ **R10.4 / T4b — the year was OPENED from the prior one, which carried its FILING STATUS,
    /// and the confirmation that the status is still right is unanswered.**
    ///
    /// §7703(a)(1) determines marital status on the **last day of the tax year**, so it is a per-year
    /// determination by statute and the year boundary is exactly when it changes. `filing_status` has
    /// no `None` to leave it unanswered in, so the opener carries it (not carrying asserts *Single*,
    /// which is worse) — and this declaration is what stops the carry from being silent.
    FilingStatusUnconfirmed,
    /// ★★★ **R10.4 / T4b — the filer answered NO: the carried filing status is not this year's.**
    ///
    /// The return would compute on the wrong status — brackets, standard deduction, every threshold —
    /// so it refuses and names where the status is changed. Answering the question again on the new
    /// status is automatic: the prompt QUOTES the status, so R10.3's `prompt_hash` re-ask rule
    /// returns it to unanswered the moment the status is edited.
    FilingStatusChanged,
    /// Form 6251 line 3 — the AMT qualified-dwelling question is live but unanswered.
    AmtQualifiedDwellingUnanswered,
    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I** — the §911/931/933 exclusion gate is unanswered on
    /// a return whose form needs modified AGI. Only reachable from TY2025 onward; TY2024's flat SALT
    /// cap never reads MAGI (D-11).
    IncomeExclusionUnanswered,
    /// Form 6251 line 3 — answered ADVERSELY ("not an AMT-qualified dwelling"). v1 does not model the
    /// §56(b)(1)(C) add-back, so computing would UNDERSTATE the tax.
    AmtNonQualifiedDwelling,
    /// Form 6251 line 2k — the AMT capital-loss-carryover question is live but unanswered.
    AmtCarryoverDeclarationUnanswered,
    /// Form 6251 line 2k — answered ADVERSELY ("my AMT carryover differs"). v1 models no divergence,
    /// so computing would UNDERSTATE the tax.
    AmtCarryoverDiverges,
    /// Form 6251 line 2l — the AMT depreciation question is live but unanswered.
    ///
    /// ★ Why this exists (whole-branch review C-2). [`ScheduleCInputs::expenses`] is a **flat filer-
    /// supplied total** — Part II's individual lines are not itemized — and Schedule C Part II **line 13
    /// is "Depreciation and section 179 expense deduction."** So the §56(a)(1) 200%-DB-vs-150%-DB
    /// adjustment rides INSIDE an accepted input, unseparated and invisible. That is structurally the
    /// same channel as the line-2k carryover: an accepted input with an uncapturable AMT twin. It gets
    /// the same remedy.
    AmtDepreciationDeclarationUnanswered,
    /// Form 6251 line 2l — answered ADVERSELY ("my AMT depreciation differs"). [`Form6251`] has no
    /// `line2l` field, so computing would UNDERSTATE the tax.
    ///
    /// [`Form6251`]: crate::tax::form6251::Form6251
    AmtDepreciationDiverges,
    /// A charitable gift/carryover to a **non-50%-organization** (Cash30/OrdinaryProp30/CapGainProp20 —
    /// private foundations etc.) needs the Pub. 526 "special 30% limit" ordering v1 doesn't implement;
    /// refuse rather than mis-limit and understate tax (review C1). Never produced by the crypto ledger.
    NonPublicCharityContribution,
    /// A claimable-as-dependent **spouse** limits the joint standard deduction (1040 Std-Deduction
    /// Worksheet), which v1 doesn't model — refuse rather than grant the full basic std (review I1).
    DependentSpouseUnsupported,
    /// W-2 box-12 code outside the inert allowlist (audit I1).
    UnsupportedBox12Code(String),
    /// Σ box-12 D/E/F/G/S elective deferrals over the §402(g) limit → taxable excess on 1040 1h (F3).
    ExcessElectiveDeferral,
    /// W-2 box 8 allocated tips (→ Form 4137).
    AllocatedTips,
    /// W-2 box 10 dependent-care benefits (→ Form 2441 Part III).
    DependentCareBenefit,
    /// 1099-INT box 9 / 1099-DIV box 13 private-activity-bond interest (AMT preference).
    PrivateActivityBondAmt,
    /// 1099-DIV box 2b/2c/2d (§1250 / §1202 / 28%-collectibles) → Schedule D Tax Worksheet (out of scope).
    UnrecapturedOrSpecialRateGain,
    /// A 1099-DIV box 1b (qualified) or box 5 (§199A) EXCEEDS its box 1a (ordinary dividends) on the same
    /// form — box 1b/box 5 are form-guaranteed SUBSETS of box 1a, so an excess is a corrupt import that
    /// would give preferential/QBI treatment to income never entered in AGI (a silent understatement,
    /// Fable IMPL-P4 r1 I4). Fail loud, like the other inconsistent-input guards (R3-M9, MFS tri-state).
    InconsistentDividendSubset(String),
    /// Foreign tax > the §904(j) $300/$600 no-Form-1116 ceiling.
    ForeignTaxOverCeiling,
    /// A single employer over-withheld Social Security (not creditable — recover from the employer).
    ///
    /// ★★ NO LONGER RAISED. i1040gi says *"you can't claim the excess on your return. The employer
    /// should adjust the tax for you"* — **not** "you can't file". The return is complete and correct
    /// without the credit, so this now yields $0 on Schedule 3 line 11 plus
    /// [`crate::tax::advisories::Advisory::ExcessSsSingleEmployerNotCreditable`], which carries the
    /// amount and the Form 843 remedy. Kept as a variant so the exhaustive cross-crate matches stay
    /// honest and any persisted value still maps.
    ///
    /// ★ When this comment was written the advisory did not exist — it asserted one, and a review
    /// caught the claim as false. The omission mattered: a filer was told nothing about money that is
    /// real and recoverable, in a codebase whose rule is that a conservative omission is permitted
    /// **only if the filer is told**.
    SingleEmployerExcessSs,
    /// **§G-22 / B11** — the scope attestation is unanswered. Silence about out-of-scope INCOME is
    /// indistinguishable from "there is none", and the failure direction is omitted §61 income.
    OtherIncomeUnanswered,
    /// **§G-28/B1b** — Form 8995-A Part I column (b) is unanswered on a return that has a trade or
    /// business. Above the §199A(d)(3) phase-in range an SSTB's QBI is excluded entirely, so an
    /// unasked "no" hands the filer a deduction the statute denies.
    SstbUnanswered,
    /// **§G-22 / B11** — the filer affirmed income this version cannot model (rental, royalty, farm,
    /// K-1, and the rest). Out of scope, and a return that silently omitted it would understate tax.
    OtherIncomeOutOfScope,
    /// §6413(c) turns on employer identity and at least one W-2 has no EIN, on a person whose
    /// aggregate box 4 exceeds the §3101(a) cap. Refuses rather than guessing — the credit is a real
    /// figure on a signed return and the wrong guess UNDERSTATES tax.
    ExcessSsEmployerUnknown,
    /// Schedule 1 line 13 HSA ACTIVITY (§223 trigger) affirmed → Form 8889 mandatory, out of scope for v1.
    /// (Renamed from `HsaPresent`: the field it reads was renamed `hsa_present → hsa_activity` in P9 §2.4 —
    /// the question is now whether a trigger fired, not mere holding.)
    HsaActivityUnsupported,
    /// P9 §2.5 (r5 I-3) — `dual_status_alien == Some(true)`. A dual-status return is out of scope for v1, and
    /// §63(c)(6)(B) zeroes a nonresident alien's standard deduction: proceeding would take the full standard
    /// deduction the statute denies (a silent understatement). VALUE-refusal, disjoint from the `None`
    /// registry loop.
    DualStatusAlienUnsupported,
    /// P9 §2.2 (Fable r2 I-3) — the §164(b)(5) sales-tax election is ON (`Some(true)`) with a $0
    /// `salt_sales_tax_amount`, and income-tax SALT (W-2 box 17/19, estimated payments, prior-year balance)
    /// would otherwise be deducted. 5a = the sales-tax amount ONLY, so the election collapses SALT to $0 — a
    /// silent loss. The symmetric twin of `SaltSalesTaxWithoutElection`.
    SalesTaxElectionWithoutAmount,
    /// P9 §3.2 (r1 I-6) — Schedule B 7a "Yes" (`foreign_accounts == Some(true)`) with a BLANK 7b
    /// (`foreign_country_names` empty/whitespace). The filed Schedule B Part III would omit the required
    /// country list. Its detail names `income import` (not `income answer` — `answer` cannot capture strings).
    ScheduleBForeignCountryMissing,
    /// Schedule 1 line 20 IRA deduction claimed → active-participant phase-out unmodeled in v1.
    IraDeductionClaimed,
    // ── Compute-dependent rows (SPEC §4.10; need the assembled income / ledger, screened in P2) ──
    /// Business-flagged crypto `Interest` income (§1402(a)(2) excludes it from SE yet it is not
    /// NIIT-sheltered → no clean v1 home, R3-I3).
    BusinessInterestIncome,
    /// The ledger has SE-eligible business crypto income but no `schedule_c` was provided — owner /
    /// description are unknowable, so v1 fails loud rather than guess (§4.4a / R3-M10 / G15).
    BusinessIncomeWithoutScheduleC,
    /// Schedule C net profit < 0 (a loss): §465 at-risk + a negative Sch 1 L3 is unsubstantiated in v1 (I2).
    ScheduleCLoss,
    /// A Schedule C with no `business_description`. **Fable P7 r2 I2.** Schedule C line A ("Principal
    /// business or profession") and Form 8995 row 1i(a) ("Trade, business, or aggregation name") both
    /// demand it, and the field is `#[serde(default)]` so an import that omits it yields `""`. Left
    /// unrefused, the filer files a Schedule C with a blank line A and a Form 8995 whose non-zero line 2
    /// totals an EMPTY column (c) — a deduction claimed for a business the return never names.
    ScheduleCNoBusinessDescription,
    /// A filer who meets all five of Form 8615's conditions → §1(g), the parent's-rate tax.
    ///
    /// ★★★ **FR-29 IS FIXED (SPEC `design/ty2025/SPEC_form8615_kiddie_tax.md`, ladder step 7).**
    /// The screen no longer reads `can_be_claimed_as_dependent_taxpayer` at all, because the IRS says
    /// dependency is not one of the conditions — `design/forms/extract/i8615--2025.txt:58-61`:
    ///
    /// > *"These rules apply whether or not the child is a dependent. These rules don't apply if
    /// > neither of the child's parents were living at the end of the year."*
    ///
    /// The five conditions are unearned income over the threshold, a filing requirement, an AGE +
    /// earned-income-support test, at least one parent alive, and a return that is not joint
    /// (`i1040gi--2025.txt:3927-3944`). btctax computes 1, 2 (assumed TRUE — SPEC §5.4) and 5, and
    /// COLLECTS 3 and 4. ★ Condition 2 is assumed, so this refusal's detail discloses the assumption
    /// rather than telling the filer they met it.
    KiddieTax,
    /// ★ **FR-29 R-1 (ladder step 3)** — Form 8615's condition 3 is UNANSWERED on a return where it
    /// decides the number: unearned income is over the §1(g) threshold, the return is not joint, and
    /// no date of birth proves the filer was 24 or older at year end. Cleared by
    /// [`crate::tax::questions::SkippableId::Form8615Condition3AgeSupport`] — or by a date of birth.
    Form8615AgeSupportUnanswered,
    /// ★ **FR-29 R-2 (ladder step 4)** — condition 3 is answered YES and condition 4 ("At least one of
    /// your parents was alive at the end of the year") is UNANSWERED. `None` only: `Some(CannotKnow)`
    /// is an ANSWER and reaches [`Self::Form8615ParentUnidentifiable`] instead.
    Form8615ParentAliveUnanswered,
    /// ★★★ **FR-29 R-4 (ladder step 6)** — the filer answered condition 4 *"cannot know"*, and has not
    /// attested that they cannot give the IRS a parent's name and address. Two different filers land
    /// here: one who has not yet been offered the second question (it becomes live only once condition
    /// 4 is answered), and one who answered that they CAN supply a name and address, for whom the
    /// IRS-request route is open and this refusal is correct and final until they use it.
    ///
    /// ★ OQ-5 is RULED (`design/OWNER_DECISIONS_2026-09-04.md`, *"OWNER RULINGS, 2026-09-05"*): the
    /// certification is NOT widened for a filer who knows their parents but is barred from contacting
    /// them. They are refused here, and the detail names the Taxpayer Advocate Service.
    Form8615ParentUnidentifiable,
    /// QBI present (REIT §199A dividends or a REIT/PTP carryforward) with taxable-income-before-QBI ABOVE
    /// the §199A(e)(2) threshold — the simplified Form 8995 no longer applies and the 8995-A phase-in is
    /// unmodeled in v1 (SPEC §4.5). Compute-dependent (needs L12 → TI-before-QBI).
    QbiAboveThreshold,
    /// **§G-28/B1b** — the filer is a PATRON of an agricultural or horticultural cooperative. Form
    /// 8995-A Part II line 14 subtracts a patron reduction figured on **Schedule D (Form 8995-A)**,
    /// which btctax does not fill; filing with line 14 blank would OVERSTATE the deduction. Note this
    /// refuses at ANY income — 8995-A's own header sends a patron to that form regardless.
    CooperativePatron,
    /// **§G-28/B1b** — a SPECIFIED SERVICE trade or business whose taxable income is INSIDE the
    /// §199A phase-in range. i8995a Exception 2: *"an applicable percentage of your SSTB is treated as
    /// a qualified trade or business, you must complete Schedule A (Form 8995-A)"* — which scales QBI,
    /// W-2 wages and UBIA before Part I. btctax does not fill that schedule, and approximating the
    /// applicable percentage would overstate the deduction.
    SstbInPhaseInRange,
    /// **§G-28/B1b** — a prior-year qualified business net LOSS carryforward on a return that files
    /// Form 8995-A. i8995a: *"If any of your trades, businesses, or aggregations have a qualified
    /// business loss for the current year **or you have a qualified business net loss carryforward from
    /// prior years**, you must complete Schedule C (Form 8995-A) before starting Form 8995-A, Part I."*
    /// btctax does not fill it. (Below the threshold the simplified Form 8995 carries the same
    /// carryforward on its own line 3, so this refuses only on the 8995-A path.)
    QbiCarryforwardNeedsSchedule8995AC,
    /// **§G-28/B1b** — Form 8995-A Part I column (e) is unanswered on a return that files a §199A form.
    /// The answer decides WHICH form is filed, so an unasked "no" prints the simplified Form 8995 for a
    /// filer the instructions send to 8995-A.
    CooperativePatronUnanswered,
    /// **§G-28/B4** — a Form 1099-B whose transactions do not all qualify for Schedule D lines 1a/8a.
    /// Those lines are available only *"for which basis was reported to the IRS and for which you have
    /// no adjustments"*; anything else needs Form 8949 with Box B/C/E/F checked and PER-TRANSACTION
    /// detail. btctax fills Form 8949 from its own crypto lot engine only and will not build a second
    /// one for securities, so this refuses rather than file totals on a line that cannot carry them.
    Form1099BNeedsForm8949,
    /// **Form 6251 must be ATTACHED** — i6251, *Who Must File*, condition 1: line 7 is greater than
    /// line 10. btctax COMPUTES the form for every return (v0.14.0+) but cannot yet file it, so such a
    /// return is refused rather than filed incomplete. Compute-dependent (needs the assembled return).
    ///
    /// NOT `amt > 0`: when line 7 exceeds line 10 the AMT foreign tax credit is figured, so line 11 can
    /// still land at $0 while the form is required. A return that clears the test has line 11 = $0, so
    /// Schedule 2 line 2 is $0 and nothing is attached.
    ///
    /// ★ The NAME is a historical misnomer — before v0.14.0 the trigger was the 1040 screening
    /// worksheet, which now gates nothing. Renaming it reopens a cross-crate exhaustive-match blast
    /// radius, so it is deferred to the Tier-2 bump (already breaking). See FOLLOWUPS G-7.
    AmtScreenTriggered,
    /// **Capital Loss Carryover Worksheet header / §1212(b)** — the joint-return sourcing declaration
    /// was never answered. Raised by the registry loop.
    ///
    /// ★ A SEPARATE variant from its adverse twin below, because that separation is an invariant here:
    /// `every_live_unanswered_declaration_refuses_with_its_own_reason` asserts that ANSWERING a
    /// question clears its unanswered reason, so a single variant covering both states cannot exist.
    /// The two are different facts anyway — "you have not told us" versus "you have told us, and it is
    /// something btctax cannot compute".
    JointReturnCarryoverDeclarationUnanswered,
    /// **Capital Loss Carryover Worksheet header / §1212(b)** — *"any capital loss carryover from the
    /// joint return can be deducted only on the return of the spouse who actually had the loss."*
    /// answered ADVERSELY (`Some(true)`).
    ///
    /// btctax stores one `Carryforward` per return and cannot split it by spouse, so an affirmative
    /// answer is not a gap in the question — it is a gap in the MODEL, and no input clears it: the
    /// filer must work the split out from the joint year's Schedule D.
    JointReturnCarryoverAttributionUnknown,
    /// **Capital Loss Carryover Worksheet header / §108(b)(2)(G)** — the canceled-debt declaration was
    /// never answered. Raised by the registry loop; see the sourcing sibling above for why the
    /// unanswered and adverse states are separate variants.
    ExcludedCanceledDebtDeclarationUnanswered,
    /// **Capital Loss Carryover Worksheet header / §108(b)(2)(G)** — *"If you excluded canceled debt
    /// from income in 2025, see Pub. 4681."* answered ADVERSELY (`Some(true)`).
    ///
    /// §108(b) requires tax ATTRIBUTE REDUCTION after an exclusion and §108(b)(2)(G) lists capital
    /// loss carryovers among the attributes; btctax models none of it, so it refuses rather than
    /// deduct and carry forward an unreduced figure.
    ///
    /// ★ **An honest DEAD END, and the detail says so.** It is the one refusal here with no cure on
    /// this year's return: the declaration is a true statement about the exclusion year, so it stays
    /// `Some(true)` however the carryover is edited, and the exclusion is reported on Form 982, which
    /// btctax does not emit. The widening review's Minor was that the old wording ("enter the reduced
    /// carryover") read as a cure and would send the filer back into the same refusal after the hand
    /// work; the reduced figure belongs to the FOLLOWING year, which btctax can file.
    ExcludedCanceledDebtAttributeReduction,
    /// ★★★ **T3a — Schedule 1-A line 5 has NO INPUT PATH, and a printed `0` would be fabricated
    /// testimony.**
    ///
    /// Line 5 is *"Qualified tip amount included in Form 1099-NEC, box 1; Form 1099-MISC, box 3; or
    /// Form 1099-K, box 1a."* `ReturnInputs` carries `w2s`, `int_1099`, `div_1099`, `g_1099` and
    /// `b_1099` — **there is no 1099-NEC, 1099-MISC or 1099-K struct anywhere in the input model**.
    /// So the line would be blank *because nothing can populate it*, which on the printed page is
    /// indistinguishable from a filer who truly had none. Under §G-11 a printed `0` is an
    /// affirmative sworn statement that the amount IS zero, so btctax's three lawful moves are
    /// collect, refuse, or genuinely blank — and "silently zero" is none of them.
    ///
    /// ★★ **The predicate is a CONJUNCTION and both limbs matter.** Not `schedule_c.is_some()`
    /// alone: a Schedule C IS the mining household, btctax's core case, so refusing on its presence
    /// would refuse every TY2025 mining return on a part the filer was told not to complete. Not the
    /// Part II claim gate alone either: lines 4a–4c are tips *"received as an employee"* while line 5
    /// is tips *"received in the course of a trade or business"*, so a W-2-only tipped employee — the
    /// form's own worked example — has no trade or business, line 5's predicate is determinately
    /// FALSE, and blank is simply correct. Gating on the part would leave Part II unreachable.
    ///
    /// ★ The backstop keeping that blank honest is `other_out_of_scope_income`, which is always live
    /// and refuses on both `None` and `Some(true)`.
    Schedule1aTipsFromTradeOrBusiness,
    /// ★★★ **T3a — Schedule 1-A line 14b has NO INPUT PATH**, the same shape one part down.
    ///
    /// Line 14b is *"Qualified overtime compensation included in Form 1099-NEC, box 1, or Form
    /// 1099-MISC, box 3."* Same conjunction, and ★ note WHICH guard holds WHICH half: the conjunct
    /// is the live guard on the **1099-NEC box 1** side, because nonemployee compensation *is*
    /// trade-or-business income and with no Schedule C there is nothing on that side to claim. The
    /// **1099-MISC box 3** side is Other Income on Schedule 1 line 8z, not Schedule C, and is held
    /// instead by `other_out_of_scope_income` — which a filer with no Schedule C must answer
    /// `Some(false)` for a return to be produced at all, so they have affirmatively sworn there was
    /// no such income and 14b is genuinely blank rather than laundered.
    Schedule1aOvertimeFromTradeOrBusiness,
    // ── ★★★ R3 / §5.1 — THE DOCUMENT CENSUS's four refusals. ────────────────────────────────────
    //
    // Each carries the ROW, never a string, so the message table lives in one place
    // (`document_census.rs`) and a new row is a compile error there rather than a missing case here.
    /// ★★★ **A live census row is `None`** — the document type was never asked about.
    ///
    /// UNANSWERED class. *"None"* and *"nobody asked"* are the same blank on the printed page and
    /// are not the same testimony; a broker that has not mailed by February is **unanswered**.
    /// Raised by the [`crate::tax::questions::FORM_QUESTIONS`] loop, like every other class-(A)
    /// declaration — one registry entry, not a new tier.
    DocumentCensusUnanswered {
        kind: crate::tax::document_census::DocumentRow,
    },
    /// ★★★ **`Some(false)` with rows transcribed** — a "no" the data contradicts.
    ///
    /// INVALID class. The filer says they received none and the return carries transcribed rows of
    /// exactly that document. One of the two is wrong and btctax cannot know which, so it refuses
    /// rather than pick; `apply` refuses the `SetField(No)` while rows exist (the
    /// `DeleteSection(ScheduleA)` I-10 precedent), so the renderer removes the rows first with a
    /// payload-confirm and nothing is ever silently deleted.
    DocumentCensusContradicted {
        kind: crate::tax::document_census::DocumentRow,
    },
    /// ★★★ **`Some(true)` with zero rows transcribed** — a declared document that was never entered.
    ///
    /// UNANSWERED class: this is precisely *"nothing ever populated it"*, the defect
    /// `blank-is-the-normal-case` names — invisible in the emitted PDF, invisible to both oracles,
    /// and invisible to any test that checks the value.
    DocumentDeclaredNotTranscribed {
        kind: crate::tax::document_census::DocumentRow,
    },
    /// ★★★ **`Some(true)` on a type btctax cannot take** — §2.2's excluded families, plus the four
    /// whose screen is task T5.
    ///
    /// UNSUPPORTED class: the data is true, btctax's scope is short, nothing is stored. The detail
    /// carries that family's own **exit sentence verbatim** and names the document, so the filer is
    /// never under-filed silently and always has somewhere to go.
    DocumentTypeUnsupported {
        kind: crate::tax::document_census::DocumentRow,
    },
    // ── ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR's refusals. ─────────────────────────────────
    /// **A live `w2_wages_without_w2` is `None`** — UNANSWERED class, raised by the registry loop.
    WagesWithoutW2Unanswered,
    /// ★★★ **The filer declared wages from an employer who issued no Form W-2.**
    ///
    /// *"Even if you don't get a Form W-2, you must still report your earnings"*
    /// (`i1040gi--2025.txt:2442-2444`). Form 1040 line 1a takes them, and btctax has no surface for
    /// wages that arrive without the document: `w2s` is a transcription of Forms W-2, and inventing
    /// a row would fabricate an employer, an EIN and a withholding figure. Refusing is the only
    /// honest answer — dropping the income would UNDERSTATE.
    WagesWithoutW2,
    /// **A live `interest_or_dividends_without_1099` is `None`** — UNANSWERED class.
    InterestOrDividendsWithout1099Unanswered,
    /// ★★★ **The filer declared interest or dividends with no 1099 behind them, and transcribed no
    /// filer's-records row.** UNANSWERED class — precisely *"nothing ever populated it"*, one level
    /// below the document census. Schedule B lines 1 and 5 take exactly what a
    /// [`crate::tax::return_inputs::ScheduleBRecord`] carries.
    FilerRecordsDeclaredNotTranscribed,
    /// **A live `state_refund_without_1099g` is `None`** — UNANSWERED class.
    StateRefundWithout1099gUnanswered,
    /// **A live `itemized_prior_year` is `None`** — UNANSWERED class.
    ItemizedPriorYearUnanswered,
    /// ★★★ **A state or local income tax refund is TAXABLE and btctax does not compute the
    /// worksheet.**
    ///
    /// §111(a)'s tax-benefit rule: a refund is income only to the extent the tax deducted produced a
    /// benefit, which the **State and Local Income Tax Refund Worksheet** measures. btctax models no
    /// part of it. So `itemized_prior_year = Some(false)` leaves Schedule 1 line 1 blank BY DECISION
    /// (no benefit ⇒ no income), and `Some(true)` refuses — reached identically from a transcribed
    /// 1099-G box 2 and from the document-less refund question, because the filer owes the same
    /// answer either way (R3/I1).
    StateAndLocalRefundWorksheetNotComputed,
    // ── ★★★ R4 / T5 — the box decisions that REFUSE. ────────────────────────────────────────────
    /// ★★★ **Form 1099-INT box 11, 12 or 13 (bond premium) is > 0.**
    ///
    /// Amortizable bond premium REDUCES interest income under §171 and is reported as a Schedule B
    /// line-1 adjustment; btctax computes no part of the §171 election or that adjustment. Dropping
    /// the figure OVERSTATES the tax silently, so refusing is both the conservative and the honest
    /// direction (Pub. 550, *Amortizable bond premium*).
    AmortizableBondPremiumNotComputed,
    /// ★★★ **Form W-2 box 13 *Statutory employee* is CHECKED.**
    ///
    /// A checked box 13 sends box 1 to **Schedule C line 1**, not to Form 1040 line 1a — the wages
    /// are business receipts against which the statutory employee deducts expenses. btctax files
    /// `w2s[].box1_wages` on line 1a, so filing this W-2 would put the wages on the wrong line and
    /// forgo the Schedule C expenses; it refuses instead.
    StatutoryEmployeeW2,
    /// ★★★ **Form 1099-G box 10 *Family leave benefits* is > 0** (the Rev. December 2026 grid).
    ///
    /// Rev. Rul. 2025-4 requires a state paid family and medical leave program to report the
    /// benefits it pays, and they are includible in gross income — reaching **Schedule 1 line 8z**
    /// (*"Other income. List type and amount"*), for which btctax models no inflow. An income box
    /// with no reader UNDERSTATES, so it fails closed.
    FamilyLeaveBenefits,
}

/// A fail-closed refusal: the reason + a human-readable detail (surfaced to the user).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub reason: RefuseReason,
    pub detail: String,
}

fn refuse(reason: RefuseReason, detail: impl Into<String>) -> Option<Refusal> {
    Some(Refusal {
        reason,
        detail: detail.into(),
    })
}

/// The §904(j) FTC ceiling for `status` (general $300; doubled only for a **joint return**). §904(j)(3)(A)
/// doubles "in the case of a joint return" — a QSS return uses MFJ rate schedules but is NOT a joint
/// return, so its ceiling is $300 (spec §4.7a: "$300 ($600 MFJ)" — MFJ only, review I2).
fn ftc_ceiling_for(p: &FullReturnParams, status: FilingStatus) -> Usd {
    match status {
        FilingStatus::Mfj => p.ftc_ceiling * dec!(2),
        _ => p.ftc_ceiling,
    }
}

/// ★ The refusal detail for R10.3's re-ask. It names the reason R12's panel prints
/// ([`crate::tax::provenance::WORDING_CHANGED_REASON`]) and the exit, because *"a refusal with no
/// exit is just a brick with better prose"*.
const WORDING_CHANGED_DETAIL: &str =
    "you answered this question, but the wording of this question changed since you answered — so \
     the answer on file was given to a different question. It has been kept in the answer log's \
     history and is no longer treated as your answer. Re-answer it with `btctax income answer` (or \
     in the tax-inputs editor); nothing else about your return has changed";

/// The label of the FIRST negative money amount in `ri`, or `None` if every captured amount is ≥ 0.
/// Every full-return input is a form-box magnitude (≥ 0); a negative is a corrupt import that could
/// offset a refusal accumulator (R2-I1). **Exhaustiveness is compiler-enforced (review R3-M1):** each
/// struct is destructured with NO `..`, so a newly-added field forces a compile error here until it is
/// classified as money (checked) or non-money (`_`). A missed money field would be a silent fail-open.
fn first_negative_amount(ri: &ReturnInputs) -> Option<&'static str> {
    let neg = |v: Usd| v < Usd::ZERO;
    // Top level — a new `ReturnInputs` field breaks this destructure until it is classified.
    let ReturnInputs {
        // ★ §G-15 — the scope questions are asked in, not a money field: nothing here to screen.
        tax_year: _,
        // §G-22/B11 — a declaration; the registry screens unanswered, and a dedicated gate screens
        // `Some(true)`. Nothing to negative-screen here: it is a yes/no, not a money field.
        other_out_of_scope_income: _,
        // ★ R3 — the document census is eighteen tri-states, no money. Its own three value rules
        //   live in `screen_document_census`; there is nothing here to negative-screen.
        documents: _,
        filing_status: _,
        // PII only — no money today. This `_` is the ONE header waiver; its exhaustiveness (a future field
        // in HouseholdHeader/Person/Dependent) is now compiler-forced by the P9 §3.3 CLASSIFIER, which
        // destructures those three with no `..` — closing the old `header: _` "false floor" (P8 review r1 F4).
        header: _,
        w2s,
        int_1099,
        div_1099,
        g_1099,
        b_1099,
        form_1098e,
        // ★ R5 / T5 — the filer's-records rows DO carry money (`amount`), screened below.
        schedule_b_filer_records,
        // ★ R3 — three tri-states about income arriving without its document, plus the
        //   prior-year-itemize gate. Yes/no, never money: the registry screens `None` and the
        //   adverse-answer rules further down screen `Some(true)`.
        w2_wages_without_w2: _,
        interest_or_dividends_without_1099: _,
        state_refund_without_1099g: _,
        itemized_prior_year: _,
        schedule_c,
        schedule_a,
        // spec 1099-DA — testimony about the broker's forms, not a money field
        broker_reporting: _,
        itemize_election: _,
        mfs_spouse_itemizes: _,
        sch1,
        // ★ Sch 1-A carries three money leaves (tips, overtime, per-vehicle interest); all three
        //   are screened below. The eligibility bools are declarations, not money.
        schedule_1a,
        payments,
        capital_loss_carryforward_in,
        capital_loss_carryforward_in_provenance: _, // CarryProvenance, not an amount
        charitable_carryover_in_provenance: _,
        carryover_includes_spouses_joint_loss: _, // a declaration, not money
        excluded_canceled_debt: _,                // a declaration, not money
        amt_carryover_same_as_regular: _,         // a declaration, not money
        amt_depreciation_same_as_regular: _,      // a declaration, not money
        charitable_carryover_in,
        qbi,
        foreign_accounts: _,
        foreign_trust: _,
        fbar_filing_required: _,
        foreign_country_names: _,
        donations_had_restrictions: _, // Option<bool>, not an amount
        charitable_cwa_obtained: _,    // Option<bool>, not an amount
        filing_form_4952: _,           // a declaration, not an amount
        // ★ Form 8960 line 9b — MONEY, and it is screened. A negative allocation would ADD to net
        //   investment income (line 12 = line 8 − line 11), which the §164(b)(6) bound below cannot
        //   see: that gate tests `> bound`, and every negative passes it.
        form_8960_line9b,
        dual_status_alien: _,
        // MAGI add-backs — refused at the worksheet's point of need, not here (D-11).
        has_income_exclusion: _, // refused at the worksheet's point of need (D-11), not here
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
        // ★ R10.3 — the answer log holds dates, prompt hashes and a two-state enum. No money leaf, so
        //   nothing here to negative-screen; a money field could never be added to it, because a record
        //   is provenance about the ASKING (`FIELD_PROVENANCE.md:400-403` forbids superseded VALUES).
        answer_log: _,
        answer_log_history: _,
        // ★ R10.4 — a year number and a declaration. No money leaf; the declaration's own refusals
        //   are the registry's (unanswered) and the adverse-answer screen further down.
        opened_from: _,
        filing_status_confirmed: _,
    } = ri;

    if form_8960_line9b.is_some_and(neg) {
        return Some("Form 8960 line 9b state/local income tax allocable to net investment income");
    }

    for w in w2s {
        let W2 {
            owner: _,
            employer: _,
            // Not a money leaf — no negative screen applies. §6413(c) reads it in `return_1040`.
            ein: _,
            box1_wages,
            box2_fed_withheld,
            box3_ss_wages,
            box4_ss_withheld,
            box5_medicare_wages,
            box6_medicare_withheld,
            box7_ss_tips,
            box17_state_tax_withheld,
            box19_local_tax,
            box12,
            box8_allocated_tips,
            box10_dependent_care,
            // A checkbox and a Treasury occupation CODE — neither is a money leaf, so no negative
            // screen applies. Box 13 is screened by `StatutoryEmployeeW2` further down.
            box13_statutory_employee: _,
            box14b_treasury_tipped_occupation_codes: _,
        } = w;
        if neg(*box1_wages) {
            return Some("W-2 box 1 wages");
        }
        if neg(*box2_fed_withheld) {
            return Some("W-2 box 2 federal withholding");
        }
        if neg(*box3_ss_wages) {
            return Some("W-2 box 3 Social Security wages");
        }
        if neg(*box4_ss_withheld) {
            return Some("W-2 box 4 Social Security withholding");
        }
        if neg(*box5_medicare_wages) {
            return Some("W-2 box 5 Medicare wages");
        }
        if neg(*box6_medicare_withheld) {
            return Some("W-2 box 6 Medicare withholding");
        }
        if neg(*box7_ss_tips) {
            return Some("W-2 box 7 Social Security tips");
        }
        if neg(*box17_state_tax_withheld) {
            return Some("W-2 box 17 state tax withheld");
        }
        if neg(*box19_local_tax) {
            return Some("W-2 box 19 local tax");
        }
        if neg(*box8_allocated_tips) {
            return Some("W-2 box 8 allocated tips");
        }
        if neg(*box10_dependent_care) {
            return Some("W-2 box 10 dependent-care benefits");
        }
        for e in box12 {
            let Box12Entry { code: _, amount } = e;
            if neg(*amount) {
                return Some("W-2 box 12 amount");
            }
        }
    }
    for i in int_1099 {
        let Form1099Int {
            payer: _,
            box1_interest,
            box2_early_withdrawal_penalty,
            box3_treasury_interest,
            box4_fed_withheld,
            box6_foreign_tax,
            box8_tax_exempt_interest,
            box9_private_activity_bond_amt,
            box10_market_discount,
            box11_bond_premium,
            box12_bond_premium_treasury,
            box13_bond_premium_tax_exempt,
            // R10.2 document identity — not money leaves, so no negative screen applies.
            payer_tin: _,
            transcribed_on: _,
        } = i;
        if neg(*box10_market_discount) {
            return Some("1099-INT box 10 market discount");
        }
        if neg(*box11_bond_premium) {
            return Some("1099-INT box 11 bond premium");
        }
        if neg(*box12_bond_premium_treasury) {
            return Some("1099-INT box 12 bond premium on Treasury obligations");
        }
        if neg(*box13_bond_premium_tax_exempt) {
            return Some("1099-INT box 13 bond premium on tax-exempt bond");
        }
        if neg(*box1_interest) {
            return Some("1099-INT box 1 interest");
        }
        if neg(*box2_early_withdrawal_penalty) {
            return Some("1099-INT box 2 early-withdrawal penalty");
        }
        if neg(*box3_treasury_interest) {
            return Some("1099-INT box 3 Treasury interest");
        }
        if neg(*box4_fed_withheld) {
            return Some("1099-INT box 4 federal withholding");
        }
        if neg(*box6_foreign_tax) {
            return Some("1099-INT box 6 foreign tax");
        }
        if neg(*box8_tax_exempt_interest) {
            return Some("1099-INT box 8 tax-exempt interest");
        }
        if neg(*box9_private_activity_bond_amt) {
            return Some("1099-INT box 9 private-activity-bond interest");
        }
    }
    for d in div_1099 {
        let Form1099Div {
            payer: _,
            box1a_ordinary,
            box1b_qualified,
            box2a_capgain_distr,
            box2b_unrecap_1250,
            box2c_section_1202,
            box2d_collectibles_28,
            box4_fed_withheld,
            box5_section_199a,
            box7_foreign_tax,
            box12_exempt_interest_dividends,
            box13_private_activity_amt,
            // R10.2 document identity — not money leaves, so no negative screen applies.
            payer_tin: _,
            transcribed_on: _,
        } = d;
        if neg(*box1a_ordinary) {
            return Some("1099-DIV box 1a ordinary dividends");
        }
        if neg(*box1b_qualified) {
            return Some("1099-DIV box 1b qualified dividends");
        }
        if neg(*box2a_capgain_distr) {
            return Some("1099-DIV box 2a capital-gain distributions");
        }
        if neg(*box2b_unrecap_1250) {
            return Some("1099-DIV box 2b unrecaptured §1250 gain");
        }
        if neg(*box2c_section_1202) {
            return Some("1099-DIV box 2c §1202 gain");
        }
        if neg(*box2d_collectibles_28) {
            return Some("1099-DIV box 2d collectibles (28%) gain");
        }
        if neg(*box4_fed_withheld) {
            return Some("1099-DIV box 4 federal withholding");
        }
        if neg(*box5_section_199a) {
            return Some("1099-DIV box 5 §199A dividends");
        }
        if neg(*box7_foreign_tax) {
            return Some("1099-DIV box 7 foreign tax");
        }
        if neg(*box12_exempt_interest_dividends) {
            return Some("1099-DIV box 12 exempt-interest dividends");
        }
        if neg(*box13_private_activity_amt) {
            return Some("1099-DIV box 13 private-activity-bond dividends");
        }
    }
    for g in g_1099 {
        let Form1099G {
            payer: _,
            box1_unemployment,
            box2_state_refund,
            box4_fed_withheld,
            box10_family_leave_benefits,
            // R10.2 document identity — not money leaves, so no negative screen applies.
            payer_tin: _,
            transcribed_on: _,
        } = g;
        if neg(*box1_unemployment) {
            return Some("1099-G box 1 unemployment compensation");
        }
        if neg(*box2_state_refund) {
            return Some("1099-G box 2 state or local income tax refund");
        }
        if neg(*box4_fed_withheld) {
            return Some("1099-G box 4 federal withholding");
        }
        if neg(*box10_family_leave_benefits) {
            return Some("1099-G box 10 family leave benefits");
        }
    }
    // R4 / §5.2 — Form 1098-E box 1, the §221 chain's only figure.
    for e in form_1098e {
        let crate::tax::return_inputs::Form1098E {
            lender: _,
            lender_tin: _,
            transcribed_on: _,
            box1_interest,
        } = e;
        if neg(*box1_interest) {
            return Some("1098-E box 1 student loan interest received by lender");
        }
    }
    // R5 — the filer's-records rows for Schedule B lines 1 / 5.
    for r in schedule_b_filer_records {
        let crate::tax::return_inputs::ScheduleBRecord {
            payer_name: _,
            payer_ssn: _,
            payer_address: _,
            amount,
            kind: _,
        } = r;
        if neg(*amount) {
            return Some("Schedule B filer's-records amount (line 1 / line 5)");
        }
    }
    // §G-28/B4 — Schedule D line 1a/8a totals. ★ A negative PROCEEDS or BASIS is not a quantity that
    // exists; a broker reports both as magnitudes. The GAIN may of course be negative, and is derived.
    for b in b_1099 {
        if neg(b.short_term_proceeds) {
            return Some("1099-B short-term proceeds (Schedule D line 1a(d))");
        }
        if neg(b.short_term_basis) {
            return Some("1099-B short-term cost basis (Schedule D line 1a(e))");
        }
        if neg(b.long_term_proceeds) {
            return Some("1099-B long-term proceeds (Schedule D line 8a(d))");
        }
        if neg(b.long_term_basis) {
            return Some("1099-B long-term cost basis (Schedule D line 8a(e))");
        }
    }
    if let Some(c) = schedule_c {
        let ScheduleCInputs {
            owner: _,
            business_description: _,
            naics_code: _,
            accounting_method: _,
            // §G-28/B1b — screened where they are NEEDED (above the §199A threshold), not here.
            qbi_w2_wages,
            qbi_ubia,
            // Two `Option<bool>` DECLARATIONS, not amounts: the SSTB checkbox is mandatory only above
            // the threshold (`screen_absolute`), and the patron flag refuses on a `yes` at any income
            // — Schedule D (Form 8995-A), which btctax does not fill.
            is_sstb: _,
            is_cooperative_patron: _,
            expenses,
            // §G-28/B3 — non-ledger Schedule C revenue. Screened below: a NEGATIVE would reduce line 1
            // and understate self-employment income, SE tax and QBI all at once.
            other_gross_receipts,
            payments_requiring_1099: _, // Option<bool>, not an amount
            will_file_required_1099: _,
        } = c;
        if neg(*expenses) {
            return Some("Schedule C expenses");
        }
        // ★ §G-28/B3 — a negative is not a quantity that exists. Schedule C line 2 ("Returns and
        //   allowances") is the form's own home for revenue given back, and btctax does not fill it;
        //   letting a negative in here would reduce gross receipts through the wrong line and
        //   UNDERSTATE income, SE tax and the §199A deduction base together.
        if neg(*other_gross_receipts) {
            return Some("Schedule C non-ledger gross receipts");
        }
        // §G-28/B1b — a negative W-2-wage or UBIA figure is not a quantity that exists. Both raise the
        // §199A(b)(2) cap, so a negative would LOWER it and overstate tax; either way it is nonsense.
        if qbi_w2_wages.is_some_and(neg) {
            return Some("Form 8995-A line 4 (allocable W-2 wages)");
        }
        if qbi_ubia.is_some_and(neg) {
            return Some("Form 8995-A line 7 (UBIA of qualified property)");
        }
    }
    if let Some(a) = schedule_a {
        let ScheduleAInputs {
            medical,
            salt_use_sales_tax: _,
            salt_sales_tax_amount,
            salt_state_estimated_payments,
            salt_prior_year_balance_paid,
            salt_real_estate,
            salt_personal_property,
            mortgage_interest_1098,
            mortgage_all_used_to_buy_build_improve: _,
            mortgage_within_debt_limit: _, // a declaration, not money
            mortgage_dwelling_is_amt_qualified: _, // a declaration, not money
            investment_interest,
            charitable,
        } = a;
        if neg(*medical) {
            return Some("Schedule A medical expenses");
        }
        if neg(*salt_sales_tax_amount) {
            return Some("Schedule A sales-tax amount");
        }
        if neg(*salt_state_estimated_payments) {
            return Some("Schedule A state estimated payments");
        }
        if neg(*salt_prior_year_balance_paid) {
            return Some("Schedule A prior-year balance paid");
        }
        if neg(*salt_real_estate) {
            return Some("Schedule A real-estate taxes");
        }
        if neg(*salt_personal_property) {
            return Some("Schedule A personal-property taxes");
        }
        if neg(*mortgage_interest_1098) {
            return Some("Schedule A mortgage interest");
        }
        if neg(*investment_interest) {
            return Some("Schedule A investment interest");
        }
        for gift in charitable {
            let CharitableGift { class: _, amount } = gift;
            if neg(*amount) {
                return Some("Schedule A charitable gift amount");
            }
        }
    }
    for item in charitable_carryover_in {
        let CharitableCarryItem {
            class: _,
            amount,
            origin_year: _,
            provenance: _,
        } = item;
        if neg(*amount) {
            return Some("charitable carryover amount");
        }
    }
    // ★★ Schedule 1-A's three money leaves. Negative amounts here are impossible on their face —
    //    a negative tip, a negative overtime figure, a negative interest payment — and every one of
    //    them INCREASES an above-the-line deduction's operand set in a direction the form never
    //    contemplates. Screened for the same reason as everything else in this function: an
    //    unscreened money field is a silent fail-open.
    {
        let Schedule1aInputs {
            tips,
            overtime,
            vehicles,
        } = schedule_1a;
        if let Some(t) = tips {
            if neg(t.qualified_tips_reported) {
                return Some("Schedule 1-A qualified tips");
            }
        }
        if let Some(o) = overtime {
            if neg(o.qualified_overtime_reported) {
                return Some("Schedule 1-A qualified overtime");
            }
        }
        for v in vehicles {
            if neg(v.interest_paid) {
                return Some("Schedule 1-A car loan interest");
            }
        }
    }

    let Schedule1Inputs {
        state_refund_taxable,
        ira_deduction_claimed,
        hsa_activity: _,
    } = sch1;
    if neg(*state_refund_taxable) {
        return Some("Schedule 1 taxable state refund");
    }
    if neg(*ira_deduction_claimed) {
        return Some("Schedule 1 IRA deduction");
    }
    let Payments {
        estimated_tax_payments,
        extension_payment,
        other_withholding,
    } = payments;
    if neg(*estimated_tax_payments) {
        return Some("estimated tax payments");
    }
    if neg(*extension_payment) {
        return Some("extension payment");
    }
    if neg(*other_withholding) {
        return Some("other withholding");
    }
    let QbiInputs {
        reit_ptp_carryforward_in,
        reit_ptp_carryforward_in_provenance: _,
        qbi_carryforward_in,
        qbi_carryforward_in_provenance: _,
    } = qbi;
    if neg(*reit_ptp_carryforward_in) {
        return Some("QBI REIT/PTP carryforward");
    }
    // ★ Form 8995 line 3 is a PARENTHESIZED box, so the input is a positive MAGNITUDE. A negative here
    // would flip the sign twice and INCREASE the deduction — the understating direction.
    if neg(*qbi_carryforward_in) {
        return Some("QBI business-loss carryforward");
    }
    let Carryforward { short, long } = capital_loss_carryforward_in;
    if neg(*short) {
        return Some("short-term capital-loss carryforward");
    }
    if neg(*long) {
        return Some("long-term capital-loss carryforward");
    }
    None
}

/// Which row of the Form 8283 restriction gate fired (see [`donation_restriction_gate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonationRestrictionGate {
    /// The filer answered **Yes** to Form 8283 line 5a, 5b or 5c — a restriction or a retained right.
    Declared,
    /// The question is UNANSWERED and the year files a Section B Form 8283, i.e. one whose lines
    /// 5a/5b/5c actually print.
    UnansweredSectionB,
}

/// ★★★ spec 1099-DA R6 fold (C-1 / M-3) — THE FORM 8283 RESTRICTION GATE, in one place.
///
/// Reg §1.170A-7: a restriction or a retained right REDUCES or DENIES the §170 deduction, and btctax
/// values every donation at full fair market value. Two rows, in this order:
///
/// - `Some(true)` — a DECLARED restriction blocks wherever an 8283 attaches, at **any** amount.
/// - `None` — an UNANSWERED question blocks only where lines 5a/5b/5c actually PRINT, i.e. a Section
///   B year. Below that the questions are never posed, so silence forgoes nothing and asserts
///   nothing (an entry is testimony; a blank is none).
///
/// **Why the premises are parameters.** The full return and the crypto slice see different things,
/// and only the *decision* is shared. The full return keys `attaches_8283` on Schedule A **line 12**
/// — the CLAIMED deduction Form 8283's own text keys on — and `section_b` on that plus the
/// §170(f)(11)(C) year aggregate over $5,000. The slice has no Schedule A at all, so it keys
/// `attaches_8283` on the year EMITTING an 8283 and `section_b` on the section its printed rows
/// carry (`forms::form_8283` splits on the same year aggregate). Sharing the decision is what stops
/// the two paths disagreeing about whether a restriction blocks a filing — the C-1 defect was
/// exactly that disagreement, with the slice's half scoped to one of its two arms.
pub fn donation_restriction_gate(
    answer: Option<bool>,
    attaches_8283: bool,
    section_b: bool,
) -> Option<DonationRestrictionGate> {
    if !attaches_8283 {
        return None;
    }
    if answer == Some(true) {
        return Some(DonationRestrictionGate::Declared);
    }
    if answer.is_none() && section_b {
        return Some(DonationRestrictionGate::UnansweredSectionB);
    }
    None
}

/// ★ spec 1099-DA R1 — the Form 1099-DA screen, at the site that holds the LEDGER (called from
/// `screen_absolute`). Liveness = the regime reports basis AND ≥ 1 exchange disposition this year.
/// Live: every (provider, cohort) key with rows must be answered, and `Mixed`/`BasisDiffers` refuse
/// naming the per-lot import as the exit. Live or not: an answer for a key with no rows, or any
/// answer on a year that is not live, refuses (`BrokerAnswerUnread`) — the tool never discards
/// testimony silently. Returns the FIRST refusal in key order so the message is deterministic.
pub fn screen_broker_reporting(
    ri: &ReturnInputs,
    state: &crate::state::LedgerState,
    year: i32,
    regime: crate::forms::InformationReturnRegime,
) -> Option<Refusal> {
    use crate::forms::{broker_key, broker_question_is_live, form_8949, BrokerReported, Cohort};
    use std::collections::BTreeMap;
    let rows = form_8949(state, year);
    let mut keys_with_rows: BTreeMap<(String, Cohort), usize> = BTreeMap::new();
    for r in &rows {
        if let Some(k) = broker_key(r) {
            *keys_with_rows.entry(k).or_insert(0) += 1;
        }
    }
    let live = broker_question_is_live(&rows, regime);
    let slot_name = |c: Cohort| match c {
        Cohort::Covered => "covered",
        Cohort::Noncovered => "noncovered",
    };
    // answers that nothing would read: a key with no rows, or any answer while not live
    for (provider, answers) in &ri.broker_reporting.0 {
        for (cohort, given) in [
            (Cohort::Covered, answers.covered),
            (Cohort::Noncovered, answers.noncovered),
        ] {
            let Some(given) = given else { continue };
            let has_rows = keys_with_rows.contains_key(&(provider.clone(), cohort));
            if !live || !has_rows {
                let why = if !regime.basis {
                    format!(
                        "TY{year}'s Form 1099-DA regime reports {} — the tool would never read this answer, and testimony it would discard is not kept",
                        if regime.proceeds { "proceeds only (no basis)" } else { "nothing" }
                    )
                } else if !live {
                    format!("TY{year} has no disposition on an exchange, so no Form 1099-DA question is asked")
                } else {
                    format!("no TY{year} Form 8949 row falls under ({provider}, {cohort:?}) — a declaration about nothing is fabricated")
                };
                return Some(Refusal {
                    reason: RefuseReason::BrokerAnswerUnread {
                        provider: provider.clone(),
                        cohort,
                        year,
                    },
                    detail: format!(
                        "broker_reporting.{provider}.{} = {given:?}: {why}. Remove the answer.",
                        slot_name(cohort)
                    ),
                });
            }
        }
    }
    if !live {
        return None;
    }
    for ((provider, cohort), n) in &keys_with_rows {
        match ri.broker_reporting.answer(provider, *cohort) {
            None => {
                return Some(Refusal {
                    reason: RefuseReason::BrokerReportingUnanswered {
                        provider: provider.clone(),
                        cohort: *cohort,
                        year,
                    },
                    detail: format!(
                        "{n} TY{year} Form 8949 row(s) were disposed on {provider} as {cohort:?} lots and no Form 1099-DA answer is recorded for them — btctax never chooses a box on its own. Answer from the physical form(s): `income import` with `[broker_reporting.{provider}] {} = \"not_reported\" | \"proceeds_only\" | \"basis_matches\" | \"basis_differs\" | \"mixed\"`, or the TUI input form; `report` lists the rows under each key.",
                        slot_name(*cohort)
                    ),
                });
            }
            Some(BrokerReported::Mixed) => {
                return Some(Refusal {
                    reason: RefuseReason::BrokerReportingMixed {
                        provider: provider.clone(),
                        cohort: *cohort,
                    },
                    detail: format!(
                        "the Form 1099-DA(s) for the {n} ({provider}, {cohort:?}) row(s) do not all say the same thing, so no single Form 8949 box is true of the set — a per-lot 1099-DA import (out of scope today) is the exit; no forms were written"
                    ),
                });
            }
            Some(BrokerReported::BasisDiffers) => {
                return Some(Refusal {
                    reason: RefuseReason::BrokerBasisDiffers {
                        provider: provider.clone(),
                        cohort: *cohort,
                    },
                    detail: format!(
                        "at least one ({provider}, {cohort:?}) row's box 1g basis differs from btctax's column (e): the row needs the broker's figure in (e) and the correction in (g) (Form 8949's own Note), which only a per-lot 1099-DA import (out of scope today) can supply; no forms were written"
                    ),
                });
            }
            Some(_) => {}
        }
    }
    None
}

/// ★★★ **R3 — THE DOCUMENT CENSUS's three VALUE rules**, in one place.
///
/// The fourth rule — a live row that is `None` — is not here: it is the
/// [`crate::tax::questions::FORM_QUESTIONS`] registry loop, so the census is one registry entry per
/// row rather than a new tier of screen (R3).
///
/// Each rule is gated on the row's own liveness, the [`crate::tax::questions::question_is_live`]
/// precedent: an ungated value-refusal is an exit-less brick, because a stale answer on a row that
/// is no longer asked cannot be cleared by the filer.
///
/// Returns the FIRST refusal in [`DocumentRow::ALL`] order, so the message is deterministic.
pub fn screen_document_census(ri: &ReturnInputs) -> Option<Refusal> {
    use crate::tax::document_census::{
        declared_rows, requires_transcription, row_is_live, DocumentRow,
    };
    for row in DocumentRow::ALL {
        let row = *row;
        if !row_is_live(ri, row) {
            continue;
        }
        let doc = row.designation();
        match ri.documents.get(row) {
            // The unanswered half belongs to the registry loop (R3: one registry entry, not a tier).
            None => continue,
            Some(true) => {
                // ★ §2.2 first: a type btctax cannot take refuses whatever its row count, because
                //   the exit is the whole answer and a row count would only distract from it.
                if let Some(exit) = row.exit_sentence() {
                    return refuse(
                        RefuseReason::DocumentTypeUnsupported { kind: row },
                        format!(
                            "you answered that you received one or more {doc}. {exit} Nothing was \
                             stored and no forms were written."
                        ),
                    );
                }
                // ★★★ TWO PREDICATES, not one. `declared_rows` says how many rows the return
                //     carries; `requires_transcription` says whether a declared document of this
                //     kind MUST appear as rows at all. A 1099-B all of whose transactions go on
                //     Form 8949, and a 1099-G reporting only box 2, are correct returns with zero
                //     rows — refusing them was the T3 seam review's I2/I3.
                if requires_transcription(row) && declared_rows(ri, row) == Some(0) {
                    // ★ The refusal names the WAY IN, not just the gap: "you declared one and none
                    //   is transcribed" is half a message, and a refusal with no exit is a brick
                    //   with better prose.
                    let route = row.entry_route().unwrap_or("enter the document");
                    return refuse(
                        RefuseReason::DocumentDeclaredNotTranscribed { kind: row },
                        format!(
                            "you answered that you received one or more {doc}, and none is \
                             transcribed on this return. A declared document with no transcription \
                             is exactly \"nothing ever populated it\" — the blank that looks \
                             identical to a correct one on the printed page. Either {route}, or \
                             change the answer to \"no\" if you received none."
                        ),
                    );
                }
            }
            Some(false) => {
                if let Some(n) = declared_rows(ri, row) {
                    if n > 0 {
                        return refuse(
                            RefuseReason::DocumentCensusContradicted { kind: row },
                            format!(
                                "you answered that you received NO {doc}, and this return carries \
                                 {n} transcribed row(s) of exactly that document. btctax cannot know \
                                 which of the two is wrong, so it refuses rather than choose: remove \
                                 the row(s) and keep the \"no\", or change the answer to \"yes\"."
                            ),
                        );
                    }
                }
            }
        }
    }
    None
}

/// ★★★ **T4 / R11 — WHAT A CALLER OF THE SCREEN HAS, so there is only ever ONE screen.**
///
/// [`screen_inputs`] is one ordered body whose early returns are semantic — `SPEC_input_form.md` §7
/// forbids refactoring it to collect refusals, because a later rule assumes an earlier one's
/// integrity. R11 adds a SECOND caller, `income import`, which must screen a TOML on a year that has
/// no [`TaxTable`] and no [`FullReturnParams`]; and the one thing that caller must not be is a
/// second copy of the rules, because *"a rule that exists in one and not the other is the defect"*.
///
/// So the body stays single and this type says what the caller HOLDS. Two axes, each a MECHANISM
/// rather than a list of rules someone must keep in step:
///
/// - **`package`** — the year's table and parameters. `None` skips exactly the rules that read one,
///   and they are the only three in the whole body: §6413(c) excess social security (from
///   `TaxTable::ss_wage_base`), §402(g) elective deferrals (`FullReturnParams::elective_deferral_limit`)
///   and the §904(j) ceiling (`FullReturnParams::ftc_ceiling`). Every other rule needs neither and
///   runs on both paths, by construction — there is no second list to fall out of step with.
/// - **`unanswered_refuses`** — whether a LIVE class-(A) declaration left `None`, or answered under
///   earlier words (R10.3), refuses here.
///
/// ★★★ **Why `unanswered_refuses` is `false` for `income import`, and why that is a brick rather
///     than a preference.** `income import` is the ONLY path that creates a committed row
///     (`answer.rs`: *"only `income import` creates one"*), and `income answer` is the ONLY path
///     that answers a declaration without hand-editing a TOML. An import that demanded every answer
///     would therefore make `income answer` unreachable for the filer it exists for — the D-8
///     recovery story, a TOML-less user facing a permanently-refusing year. R11's own enumeration of
///     what runs at import lists the census invariants, the unsupported-row refusals and
///     `NegativeAmount`, and does not name the unanswered tier.
///
/// ★ The census's own VALUE rules are NOT in that tier and DO refuse at import: a declared document
///   with nothing transcribed, a *"no"* beside transcribed rows, and an unsupported type are each
///   unfixable by `income answer` (it captures booleans and dates, never a document row), so
///   refusing at import is the filer's real exit, not a wall.
#[derive(Clone, Copy)]
pub struct ScreenTier<'a> {
    /// The year's package, when the caller has one.
    pub package: Option<(&'a TaxTable, &'a FullReturnParams)>,
    /// Whether an unanswered (or wording-changed) live class-(A) declaration refuses here.
    pub unanswered_refuses: bool,
}

/// Screen the **input-screenable** refuse-guard rows (SPEC §4.10). Returns the FIRST [`Refusal`] found,
/// or `None` if nothing input-screenable trips (the compute/ledger-dependent rows are checked later).
///
/// The COMMIT gate: every tier runs, because the caller has the year's package.
pub fn screen_inputs(ri: &ReturnInputs, tbl: &TaxTable, p: &FullReturnParams) -> Option<Refusal> {
    screen_inputs_tiered(
        ri,
        ScreenTier {
            package: Some((tbl, p)),
            unanswered_refuses: true,
        },
    )
}

/// ★★★ **R11 — the screen `income import` runs BEFORE it writes, on a year with no package.**
///
/// Until T4, `income import` wrote a committed, UNSCREENED row on every params-less year — so a TOML
/// carrying `documents.k1 = true` was stored and poisoned the year at `resolve.rs` precedence 1
/// (`SPEC_input_surface.md` §3.2) until `report` finally said so. This is the same body
/// [`screen_inputs`] runs, on the two tiers a params-less importer can honestly run: see
/// [`ScreenTier`] for why those two and not others.
#[must_use]
pub fn screen_param_free(ri: &ReturnInputs) -> Option<Refusal> {
    screen_inputs_tiered(
        ri,
        ScreenTier {
            package: None,
            unanswered_refuses: false,
        },
    )
}

/// The one screen body. See [`ScreenTier`]; the rule ORDER is unchanged from the pre-T4
/// `screen_inputs`, so a caller holding the package gets byte-identical behaviour.
pub fn screen_inputs_tiered(ri: &ReturnInputs, tier: ScreenTier<'_>) -> Option<Refusal> {
    // Data integrity FIRST: any negative money is a corrupt import — refuse before any accumulation, so a
    // negative can never offset a §402(g) / §904(j) threshold into passing (R2-I1 / M4, now one gate).
    if let Some(field) = first_negative_amount(ri) {
        return refuse(
            RefuseReason::NegativeAmount(field.to_string()),
            format!("{field} is negative — every full-return money amount is a form-box magnitude (≥ 0); fix the import"),
        );
    }

    // ★★★ §G-28/B4 — Schedule D lines 1a/8a carry TOTALS only for transactions the form itself admits:
    //     *"for which basis was reported to the IRS and for which you have no adjustments"*. Both limbs,
    //     answered by the filer. `None` (never asked) and `Some(false)` both refuse — a row that is not
    //     an affirmative `yes` fails closed, so an omission can never become a claim.
    //
    //     ★ Gated on the row carrying an AMOUNT: an all-zero 1099-B row asserts nothing and reports
    //       nothing, so demanding a confirmation for it would be a refusal with no purpose.
    // ★★★ T3a — Schedule 1-A lines 5 and 14b read 1099-NEC / 1099-MISC / 1099-K, and btctax has no
    //     struct for any of them. Where the line's own predicate could be TRUE, refuse rather than
    //     print a zero nobody swore to. Both are CONJUNCTIONS: the part must be claimed AND the
    //     filer must have a trade or business, because a W-2-only tipped employee has no trade or
    //     business and blank is then simply correct. See the RefuseReason docs for why neither limb
    //     alone is right — each was tried and each refused a population it should not have.
    if ri.schedule_c.is_some() {
        if ri.schedule_1a.tips.is_some() {
            return refuse(
                RefuseReason::Schedule1aTipsFromTradeOrBusiness,
                "Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC box 1,                  1099-MISC box 3 or 1099-K box 1a. btctax has no input for those forms, and this                  return has a trade or business, so the amount cannot be established. Entering                  nothing would look identical to having none."
                    .to_string(),
            );
        }
        if ri.schedule_1a.overtime.is_some() {
            return refuse(
                RefuseReason::Schedule1aOvertimeFromTradeOrBusiness,
                "Schedule 1-A line 14b asks for qualified overtime reported on Form 1099-NEC box 1                  or 1099-MISC box 3. btctax has no input for those forms, and this return has a                  trade or business, so the amount cannot be established."
                    .to_string(),
            );
        }
    }

    for b in &ri.b_1099 {
        let carries_totals = b.short_term_proceeds > Usd::ZERO
            || b.short_term_basis > Usd::ZERO
            || b.long_term_proceeds > Usd::ZERO
            || b.long_term_basis > Usd::ZERO;
        if carries_totals && b.basis_reported_and_no_adjustments != Some(true) {
            let who = if b.payer.trim().is_empty() {
                "(unnamed broker)"
            } else {
                b.payer.trim()
            };
            return refuse(
                RefuseReason::Form1099BNeedsForm8949,
                format!(
                    "the Form 1099-B from {who} carries totals, but Schedule D lines 1a and 8a accept \
                     totals ONLY for transactions \"for which basis was reported to the IRS and for \
                     which you have no adjustments\". Confirm BOTH by setting \
                     `basis_reported_and_no_adjustments = true` on that 1099-B and re-running \
                     `btctax income import`. If either is untrue, those transactions belong on Form \
                     8949 with Box B, C, E or F checked and one row per sale — btctax fills Form 8949 \
                     from its own crypto lot engine only, and will not report securities it cannot \
                     itemize"
                ),
            );
        }
    }

    // ★★ NO SSN GATE HERE, DELIBERATELY. A malformed SSN used to refuse the whole computation, which
    // meant one typo in an identity field blocked `report`, `optimize`, `what-if` and the TUI — none of
    // which read an SSN. That was strictly WORSE than an EMPTY SSN, which has always been allowed
    // through to the report. The identity boundary is the FILABLE PACKET, and it is already closed:
    // `ReturnHeader::build` returns `HeaderError::Ssn(SsnError::{Missing,NotDigits,WrongLength})`, so a
    // typo still cannot reach a printed comb cell. Screening it twice only cost the filer their numbers.

    // ★ P9 §3.2 — THE REGISTRY LOOP. Placed after the integrity gates (negative money, malformed SSN) and
    // before every value-dependent rule (r1 M-2). This is the ONLY unanswered-declaration screen: every
    // live class-(A) question that is `None` refuses here, deriving its reason + detail + liveness from the
    // single [`FORM_QUESTIONS`] list. It replaces four hand-written blocks (dependent ×2, MFS-itemizes,
    // Schedule B Part III) and `schedule_b_part3_unanswered` — the latter was circular (§2.9). Refusal
    // PRECEDENCE is explicitly not contract: on a multi-defect return the reported reason may differ from
    // the pre-P9 order.
    // ★★★ T4/R11 — THE WHOLE LOOP IS THE UNANSWERED TIER, and `income import` runs without it (see
    //     [`ScreenTier`]: the import is the only row-creating path and `income answer` the only
    //     answering one, so demanding the answers here would make answering unreachable).
    if tier.unanswered_refuses {
        for q in crate::tax::questions::FORM_QUESTIONS {
            if !(q.live)(ri) {
                continue;
            }
            if (q.get)(ri).is_none() {
                return refuse(q.unanswered.clone(), q.unanswered_detail);
            }
            // ★★★ **R10.3 — AN ANSWER GIVEN UNDER EARLIER WORDS DOES NOT STAND UNDER LATER ONES.**
            //
            // The `prompt_hash` exists so *which words were asked* is on record, and this is its ONE
            // reader on the refusal path: a class-(A) record whose hash no longer matches the prompt the
            // filer would be shown TODAY is refused as UNANSWERED. The case is not exotic — R11's Sep–Dec
            // calendar makes it ordinary: a prompt edited in a November fold, sitting under an answer
            // given in September on the same year's draft.
            //
            // ★ **An ABSENT record is NOT a mismatch.** Only a record that exists and disagrees refuses;
            //   a leaf answered before the log existed (or through a surface that does not record) keeps
            //   its value. Treating "no record" as unanswered would refuse every return in the corpus and
            //   would be asserting provenance nobody ever collected.
            //
            // ★★★ **`prompt_text`, NOT the static `prompt`** — the same correction
            //     [`crate::tax::provenance::current_prompt`] already carries in terms: *"a question
            //     whose subject is a value ON the return is asked in words that quote it, and those
            //     are the words that must be hashed."* `record_answer` hashes what was SHOWN, so
            //     comparing against the static fallback made every RENDERED question look
            //     wording-changed the instant it was answered — a refusal that fires on a correct
            //     answer. It was latent until T5 gave two more questions rendered prompts.
            let key = crate::tax::provenance::AnswerKey::Question(q.id);
            if crate::tax::provenance::answer_status(ri, &key, &q.prompt_text(ri))
                == crate::tax::provenance::AnswerStatus::WordingChanged
            {
                return refuse(q.unanswered.clone(), WORDING_CHANGED_DETAIL);
            }
        }
    }

    // ★★★ R3 — THE DOCUMENT CENSUS's three VALUE rules, immediately after the registry loop that
    //     owns its unanswered half. Placed here so a census answer is screened before every
    //     value-dependent rule below: a filer holding a Form 1099-R is told so, rather than meeting
    //     a §402(g) or SALT message about a return that was never fileable.
    if let Some(r) = screen_document_census(ri) {
        return Some(r);
    }

    // ★★★ R3 — THE DOCUMENT-LESS INCOME DOOR's VALUE rules, immediately after the census's own,
    //     because they are the census's other half: *"a census `No` never closes income the
    //     instructions say to report WITHOUT the document."* Each is gated on its question's own
    //     registry liveness, so a stale answer on a row that is no longer asked is never an
    //     exit-less brick.
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::WagesWithoutW2Question,
        ri,
    ) && ri.w2_wages_without_w2 == Some(true)
    {
        return refuse(
            RefuseReason::WagesWithoutW2,
            "you answered that you received wages, salary or tips from an employer who issued no \
             Form W-2. The Form 1040 instructions are explicit — \"Even if you don't get a Form W-2, \
             you must still report your earnings\" — and those earnings belong on Form 1040 LINE 1a. \
             btctax takes line 1a only from transcribed Forms W-2, so it has nowhere to put wages \
             that arrive without the document, and inventing a row would fabricate an employer, an \
             EIN and a withholding figure on a return signed under §6065. Report the whole return \
             with a preparer, or file it yourself with those earnings on line 1a",
        );
    }
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::InterestOrDividendsWithout1099,
        ri,
    ) && ri.interest_or_dividends_without_1099 == Some(true)
        && ri.schedule_b_filer_records.is_empty()
    {
        return refuse(
            RefuseReason::FilerRecordsDeclaredNotTranscribed,
            "you answered that you received taxable interest or dividends for which no Form \
             1099-INT or 1099-DIV was issued, and none is entered. Schedule B asks for exactly what \
             is missing — the payer's name and the amount (line 1 for interest, line 5 for ordinary \
             dividends), plus the buyer's SSN and address if it is a seller-financed mortgage. \
             Enter them as `[[schedule_b_filer_records]]` tables through `btctax income import`, or \
             in the \"Interest and dividends from your own records\" section of the tax-inputs \
             form — or change the answer to \"no\" if there were none.",
        );
    }
    // ★★★ R3/I1 + R4 — the STATE AND LOCAL INCOME TAX REFUND WORKSHEET, reached identically from a
    //     transcribed 1099-G box 2 and from the document-less refund question. §111(a) is why the
    //     gate and not the answer decides: a refund is income only to the extent the deduction
    //     produced a tax benefit, so a filer who did NOT itemize last year owes nothing and their
    //     Schedule 1 line 1 is blank BY DECISION rather than by omission.
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::ItemizedPriorYear,
        ri,
    ) && ri.itemized_prior_year == Some(true)
    {
        return refuse(
            RefuseReason::StateAndLocalRefundWorksheetNotComputed,
            "you received a refund, credit or offset of state or local income taxes and you \
             ITEMIZED on your prior-year return, so §111(a)'s tax-benefit rule makes some or all of \
             it income on Schedule 1 line 1. How much is decided by the STATE AND LOCAL INCOME TAX \
             REFUND WORKSHEET in the Form 1040 instructions, which btctax does not compute — it \
             needs last year's Schedule A, its SALT cap, the standard deduction you could have \
             taken and the §164(b)(6) limitation. btctax refuses rather than guess a figure in the \
             understatement direction. Work the worksheet by hand and file with a preparer, or \
             answer \"no\" if you did not itemize in the year you paid the tax — then none of the \
             refund is taxable and Schedule 1 line 1 is correctly blank",
        );
    }

    // ★★★ THE CAPITAL LOSS CARRYOVER WORKSHEET'S TWO HEADER CONDITIONS, ANSWERED ADVERSELY.
    //     VALUE-refusals (`Some(true)`), disjoint from the unanswered loop above and gated on the
    //     SAME registry liveness that half uses — a stale `Some(true)` on a return that no longer
    //     carries a carryforward must not be an exit-less brick.
    //
    // ★ Neither is a question btctax could have answered for the filer, and neither is one it can
    //   act on once answered YES: the first needs a per-spouse split of a single stored figure, the
    //   second needs §108(b) attribute reduction. So the refusal is the whole remedy, and it names
    //   the hand-work rather than pretending an input clears it.
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::CarryoverIncludesSpousesJointLoss,
        ri,
    ) && ri.carryover_includes_spouses_joint_loss == Some(true)
    {
        return refuse(
            RefuseReason::JointReturnCarryoverAttributionUnknown,
            "you declared that part of your capital-loss carryover came from a JOINT return for a \
             year you are now filing separately from, and was your spouse's loss. The Capital Loss \
             Carryover Worksheet's header is explicit: such a carryover \"can be deducted only on \
             the return of the spouse who actually had the loss\" (§1212(b)). btctax stores ONE \
             carryover figure per return and has no way to split it by spouse, so filing would \
             deduct — and carry forward — a loss that may not be yours. Work the split out by hand \
             from the joint year's Schedule D and enter only YOUR share",
        );
    }
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::ExcludedCanceledDebt,
        ri,
    ) && ri.excluded_canceled_debt == Some(true)
    {
        return refuse(
            RefuseReason::ExcludedCanceledDebtAttributeReduction,
            "you declared that you excluded cancelled or forgiven debt from income, and you are \
             carrying a capital-loss carryforward. The Capital Loss Carryover Worksheet's header \
             sends you to Pub. 4681, because §108(b) requires you to REDUCE tax attributes after \
             such an exclusion and §108(b)(2)(G) lists capital loss carryovers among them. btctax \
             models no part of §108(b), so the carryover it would deduct and carry forward is too \
             large. btctax CANNOT FILE THIS YEAR for you — the answer above is a true statement \
             about this year, so it stays Yes however you edit the carryover, and the exclusion \
             itself is reported on Form 982, which btctax does not produce. Work this year out by \
             hand (Pub. 4681, Form 982). The REDUCED carryover Pub. 4681 leaves you with is what \
             carries into the following year, and btctax can file that year once its own answer here \
             is No",
        );
    }

    // (c) foreign trust → Form 3520. VALUE-refusal (`Some(true)`); disjoint from the unanswered loop above.
    if ri.foreign_trust == Some(true) {
        return refuse(
            RefuseReason::ForeignTrust,
            "a foreign trust requires Form 3520, which is out of scope for v1",
        );
    }

    // ── Form 6251's three ADVERSE answers. VALUE-refusals (`Some(false)` — all three are
    //    neutral at TRUE, see `FormQuestion::neutral`), disjoint from the unanswered loop above.
    //    ★ We mirror the mixed-use-mortgage exemplar only on the UNANSWERED half and deliberately
    //    DIVERGE here: a zeroed Schedule A line 8a is conservative, but a missing AMT add-back is not.
    // ★ All three are gated by the SAME liveness predicate their UNANSWERED half uses, read from the
    //   registry via `question_is_live` rather than re-derived here. Ungated, a stale `Some(false)` is
    //   an exit-less brick on a return whose add-back is structurally $0 — the filer cannot clear an
    //   answer to a question that is no longer asked. (i6251 line 3 is itself conditioned on having
    //   **deducted** the interest, so the gate is faithful to the form, not just kind.)
    // ★★★ SCHEDULE D LINE 20 / SCHEDULE A LINE 9 — the Form 4952 answer, and its BOUND.
    //
    // Two ways to land here, one refusal:
    //
    //   (a) the filer says they ARE filing Form 4952. Line 20 is then "No", which routes to the
    //       SCHEDULE D TAX WORKSHEET — and btctax fills neither that worksheet nor Form 4952.
    //       Checking "Yes" anyway would understate: the SDTW subtracts Form 4952 line 4g at line 4.
    //
    //   (b) the filer says they are NOT, but their Schedule A line 9 amount BREAKS i4952's own
    //       exception — *"You don't have to file Form 4952 if … your investment interest expense is
    //       not more than your investment income from interest and ordinary dividends minus any
    //       qualified dividends"*. Above that, Form 4952 IS required whatever the filer answered, and
    //       §163(d)(1) caps the deduction at net investment income, which btctax does not compute. A
    //       BOUND on a collected value, not a computation — deducting the excess would UNDERSTATE.
    //
    // ★ Only the FIRST of i4952's three exception conditions is checkable here: the other two ("no
    //   other deductible investment expenses", "no disallowed investment interest carried over") are
    //   facts btctax never sees, which is why the PROMPT enumerates all three and defaults to Yes.
    //   This bound catches the one a wrong answer would leave visible in the numbers.
    if ri.filing_form_4952 == Some(true) {
        return refuse(
            RefuseReason::Form4952Required,
            "you are filing Form 4952 (Investment Interest Expense Deduction), so Schedule D line 20 \
             — \"Are lines 18 and 19 both zero or blank and you are not filing Form 4952?\" — is \
             answered NO, which sends your tax to the SCHEDULE D TAX WORKSHEET. btctax fills neither \
             Form 4952 nor that worksheet, and the worksheet is not a formatting difference: its \
             line 4 subtracts Form 4952 line 4g, so computing the return on the Qualified Dividends \
             and Capital Gain Tax Worksheet instead would UNDERSTATE your tax. File this year by \
             hand, or remove the investment interest if you are not in fact claiming it",
        );
    }
    // ★★★ THE LINE-9 i4952 BOUND USED TO STAND HERE AND WAS MOVED TO `screen_absolute` (phase-2
    //     review R5a, Critical). It refuses on a Schedule A DEDUCTION being over-claimed, but
    //     `screen_inputs` sees only inputs — it cannot see the §63(e) election, which
    //     `assemble_absolute` computes. So it refused filers who take the STANDARD deduction, where
    //     line 9 never prints and nothing is sworn. The population is not hypothetical: it is the
    //     crypto-margin renter, this product's core audience — bitcoin yields no interest or
    //     ordinary dividends, so the ceiling is ~$0 and ANY line-9 entry with a truthful "not filing
    //     4952" was refused, including when SALT + margin interest lose to the standard deduction and
    //     the correct return claims nothing at all. See `screen_absolute`.

    // ★★★ §163(h)(3)(B) — the ACQUISITION-DEBT CEILING, answered adversely. Same shape as the three
    //     Form 6251 declarations below: gated on the registry liveness so a stale `Some(false)` on a
    //     Schedule A that no longer reports 1098 interest is not an exit-less brick.
    //
    // ★★ THE MESSAGE NAMES BOTH FAILURE DIRECTIONS ON PURPOSE. This branch exists because *neither*
    //    number btctax can produce is filable, and a refusal that named only one of them would read as
    //    an invitation to take the other. i1040sca Line 8a: *"Only enter on line 8a the deductible
    //    mortgage interest and points that were reported to you on Form 1098"* — a determinate NONZERO
    //    worksheet output. A printed $0 transcribes no instruction, and unlike the mixed-use zero it
    //    has no line-8 checkbox disclosing it (see `ADJUDICATION-2026-08-21.md`, D3).
    // ★★★ THE §163(h)(3)(B) OVER-LIMIT REFUSAL USED TO STAND HERE AND WAS MOVED TO
    //     `screen_absolute` (phase-2 review R2, Critical). Same root cause as the line-9 bound above:
    //     it conditions a Schedule A DEDUCTION, but `screen_inputs` cannot see the §63(e) election.
    //     It therefore refused the December-closing jumbo homebuyer — one month of 1098 interest on a
    //     $1M post-2017 loan, itemized total under the MFJ standard deduction — for whom line 8a
    //     never prints. That filer had NO honest answer: `None` refused as unanswered, `Some(false)`
    //     refused here, and `Some(true)` would have been false testimony under §6065.
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::AmtQualifiedDwelling,
        ri,
    ) && ri
        .schedule_a
        .as_ref()
        .is_some_and(|a| a.mortgage_dwelling_is_amt_qualified == Some(false))
    {
        return refuse(
            RefuseReason::AmtNonQualifiedDwelling,
            "you declared that the mortgaged dwelling is NOT an AMT-qualified dwelling, so Form 6251 \
             line 3 must add that deducted interest back (i6251, Line 3 — a houseboat or recreational \
             vehicle is never AMT-qualified). v1 does not model the §56(b)(1)(C) add-back, and computing \
             without it would UNDERSTATE your tax",
        );
    }
    // ★★★ R10.4 / T4b seam review I-1 — the CARRIED filing status was answered NO.
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::FilingStatusConfirmed,
        ri,
    ) && ri.filing_status_confirmed == Some(false)
    {
        return refuse(
            RefuseReason::FilingStatusChanged,
            "you answered that the filing status carried from the prior year is NOT your filing \
             status for this one. Every bracket, the standard deduction and every threshold on this \
             return compute from it, so it cannot be filed on last year's. Change the status — in \
             the tax-inputs form's Household section, or `filing_status` in the `income import` TOML \
             — and the confirmation is asked again on the new status",
        );
    }
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::AmtCarryoverSameAsRegular,
        ri,
    ) && ri.amt_carryover_same_as_regular == Some(false)
    {
        return refuse(
            RefuseReason::AmtCarryoverDiverges,
            "you declared that your AMT capital-loss carryover differs from the regular-tax one, so Form \
             6251 line 2k must add the difference back. btctax tracks only the regular figure and models \
             no divergence, so computing would UNDERSTATE your tax",
        );
    }
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::AmtDepreciationSameAsRegular,
        ri,
    ) && ri.amt_depreciation_same_as_regular == Some(false)
    {
        return refuse(
            RefuseReason::AmtDepreciationDiverges,
            "you declared that the depreciation inside your Schedule C expenses differs for the AMT, so \
             Form 6251 line 2l must add the difference back. btctax accepts Schedule C expenses only as a \
             flat total — it never sees the depreciation line, let alone its AMT twin — so computing \
             would UNDERSTATE your tax",
        );
    }

    // ★ P9 §2.5 (r5 I-3) — a truthful dual-status "yes" is UNSUPPORTED. VALUE-refusal (`Some(true)`);
    // WITHOUT it a "yes" computes, taking the standard deduction §63(c)(6)(B) denies a nonresident alien.
    // ★★★ §G-22 / B11 — the filer AFFIRMED income this version cannot model.
    if ri.other_out_of_scope_income == Some(true) {
        return refuse(
            RefuseReason::OtherIncomeOutOfScope,
            "you answered YES to something this version cannot model — income it never asked about \
             (rent, royalties, a farm, a K-1, tips, gambling, alimony, an uncaptured business), an \
             INCENTIVE STOCK OPTION exercise you still held at year end (Form 6251 line 2i, from your \
             Form 3921), or another alternative-minimum-tax item. btctax models Form 6251 lines 2, 2a \
             and 2b only, so any other Part I add-back would print as ZERO — and because \
             `must_attach` tests line 7 against line 10, a missing add-back would also stop the AMT \
             screen from firing at all. It refuses rather than file a return that understates on a \
             line it cannot see. Remove that item and file the rest yourself"
        );
    }
    if ri.dual_status_alien == Some(true) {
        return refuse(
            RefuseReason::DualStatusAlienUnsupported,
            "you were a dual-status alien — v1 does not compute a dual-status return (§63(c)(6)(B) zeroes a \
             nonresident alien's standard deduction), so it refuses rather than over-deduct",
        );
    }

    // ★ P9 §3.2 (r1 I-6) — Schedule B 7a "Yes" with a blank 7b (country names). The exit is `income import`
    // (a TOML re-import), never `income answer` — `answer` captures bools and dates, never strings.
    if ri.foreign_accounts == Some(true) && ri.foreign_country_names.trim().is_empty() {
        return refuse(
            RefuseReason::ScheduleBForeignCountryMissing,
            "you declared a foreign financial account (Schedule B line 7a), but line 7b (the country \
             name(s)) is blank — add `foreign_country_names` to the TOML and re-run `btctax income import`",
        );
    }

    // Schedule A §164(b)(5) SALT: a sales-tax amount with the election OFF is an input error — fail loud
    // rather than silently drop it (R3-M9).
    if let Some(a) = &ri.schedule_a {
        if a.salt_sales_tax_amount > Usd::ZERO && a.salt_use_sales_tax != Some(true) {
            return refuse(
                RefuseReason::SaltSalesTaxWithoutElection,
                "a Schedule A sales-tax amount is set but the §164(b)(5) sales-tax election is off — turn \
                 the election on (5a = sales tax) or clear `salt_sales_tax_amount`",
            );
        }
        // ★ P9 §2.2 (Fable r2 I-3) — the SYMMETRIC twin: the election is ON with a $0 amount, so 5a = $0,
        // while income-tax SALT (W-2 box 17/19 + estimates + prior-year balance) would otherwise be
        // deducted. The election silently collapses the whole SALT deduction — fail loud rather than lose it.
        // ★ The income-tax SALT set is the SHARED `income_tax_salt` derivation (return_1040), not a second
        // copy — so the guarded set cannot drift from the set `salt_line_5a` actually deducts (r3 MINOR-1).
        if a.salt_use_sales_tax == Some(true)
            && a.salt_sales_tax_amount == Usd::ZERO
            && crate::tax::return_1040::income_tax_salt(ri, a) > Usd::ZERO
        {
            return refuse(
                RefuseReason::SalesTaxElectionWithoutAmount,
                // ★ T4 fold, seam review M-2: the `income answer` exit is GONE. This rule is in the
                //   import tier now (`ScreenTier`), and on a params-less year with no committed row
                //   `income answer` refuses — `income import`, the command that just refused, is the
                //   only path that could have created the row it would be answered on. Both exits
                //   named here are edits to the file the filer is holding.
                "the §164(b)(5) sales-tax election is ON but `salt_sales_tax_amount` is $0, so Schedule A \
                 line 5a would be $0 and your state/local income taxes (W-2 box 17/19 withholding, \
                 estimates, prior-year balance) drop out — enter the amount, or clear \
                 `salt_use_sales_tax` to deduct income taxes instead, and re-run `btctax income import`",
            );
        }
    }

    // (§63(c)(6) MFS-spouse-itemizes, D-8 dependent-taxpayer, and dependent-spouse UNANSWERED checks are now
    //  the registry loop above — the ONLY copy of each liveness predicate.)

    // §170(b) non-50%-org charitable classes need the Pub. 526 "special 30% limit" ordering v1 doesn't
    // implement — refuse rather than mis-limit / understate tax (review C1). Checks both current gifts and
    // carryover-in; never produced by the crypto ledger (which supplies only 50%-org classes).
    let is_non50org = |c: CharitableClass| {
        matches!(
            c,
            CharitableClass::Cash30
                | CharitableClass::OrdinaryProp30
                | CharitableClass::CapGainProp20
        )
    };
    let non50_gift = ri
        .schedule_a
        .as_ref()
        .is_some_and(|a| a.charitable.iter().any(|g| is_non50org(g.class)));
    let non50_carry = ri
        .charitable_carryover_in
        .iter()
        .any(|c| is_non50org(c.class));
    if non50_gift || non50_carry {
        return refuse(
            RefuseReason::NonPublicCharityContribution,
            "a charitable contribution to a non-50%-organization (e.g. a private foundation) is out of scope \
             for v1 — its §170(b) special-30%-limit ordering is unmodeled",
        );
    }

    // A claimable-as-dependent SPOUSE limits the joint standard deduction (1040 Std-Deduction Worksheet),
    // which v1 doesn't model (the spouse flag is otherwise unconsumed) — refuse rather than grant the full
    // basic std and understate tax (review I1). Narrow/usually-invalid input (a claimable spouse generally
    // can't file jointly).
    // (The D-8 dependent-taxpayer and dependent-spouse UNANSWERED checks are the registry loop above.)
    // A claimable-as-dependent SPOUSE (`Some(true)`) is a VALUE-refusal (it limits the joint standard
    // deduction, unmodeled) — disjoint from the unanswered loop.
    if ri.header.can_be_claimed_as_dependent_spouse == Some(true) {
        return refuse(
            RefuseReason::DependentSpouseUnsupported,
            "a claimable-as-dependent spouse is out of scope for v1 — it limits the joint standard deduction",
        );
    }

    // A Spouse-owned item is only coherent on a joint (MFJ) return; on Single/HoH/MFS/QSS the spouse's
    // income is not on this return. Refuse before the per-owner §402(g) accumulation so a mislabeled
    // `owner` cannot split one person's deferrals into two under-limit buckets (R2-I2).
    if ri.filing_status != FilingStatus::Mfj {
        let spouse_w2 = ri.w2s.iter().any(|w| w.owner == Owner::Spouse);
        let spouse_sc = ri
            .schedule_c
            .as_ref()
            .is_some_and(|c| c.owner == Owner::Spouse);
        if spouse_w2 || spouse_sc {
            return refuse(
                RefuseReason::SpouseOwnerWithoutJointReturn,
                "a spouse-owned W-2/Schedule C is only valid on a joint (MFJ) return — check the `owner` tag or the filing status",
            );
        }
    }

    // ★ Fable P7 r2 I2 — a business the return does not NAME cannot be filed. Schedule C line A and
    // Form 8995 row 1i(a) both require it, and `business_description` is `#[serde(default)]`, so an
    // import that simply omits the key produces "". The forms would then be facially incomplete: a
    // Schedule C with a blank line A, and a Form 8995 claiming a §199A deduction over an empty column.
    if let Some(c) = &ri.schedule_c {
        if c.business_description.trim().is_empty() {
            return refuse(
                RefuseReason::ScheduleCNoBusinessDescription,
                "the Schedule C has no `business_description` — Schedule C line A and Form 8995 row 1i(a) \
                 both require the name of the trade or business the return is filing (and claiming a \
                 §199A deduction) for",
            );
        }
    }

    // W-2 rows: box-12 allowlist + §402(g) deferral cap + box 8/10. (The single-employer excess-SS
    // guard is gone — see the §6413(c) block below, which refuses only on UNKNOWN employer identity.)
    // ★ T4/R11 — the §3101(a)/§6413(c) cap is a TaxTable figure, so this rule is one of the three
    //   that wait for the year's package (the gate below is `if let Some(…) = tier.package`, which
    //   is what `the_screen_tiers_are_read_off_the_source` reads to derive the tier census).
    // §402(g)(1) limits an INDIVIDUAL's elective deferrals — accumulate PER OWNER (each spouse on a joint
    // return gets its own limit; review I1), refusing iff any one person exceeds it. Amounts are already
    // guaranteed ≥ 0 by the negative screen above, so no per-entry clamp is needed.
    // ★★★ §6413(c) / Schedule 3 line 11 — the excess-SS credit turns on EMPLOYER IDENTITY.
    //
    // i1040gi: *"If you, or your spouse if filing a joint return, had **more than one employer** for
    // 2024 and total wages of more than $168,600 … You can take a credit … in excess of $10,453.20.
    // But if **any one employer** withheld more than $10,453.20, you can't claim the excess on your
    // return. The employer should adjust the tax for you."*
    //
    // ★★ The old guard here refused whenever ONE W-2's box 4 exceeded the cap — a proxy for employer
    // identity it did not have, and wrong in both directions. It **refused a return the instructions
    // say is fileable** (the credit is simply $0; the employer adjusts), while letting a filer with
    // several W-2s from ONE employer claim a credit they are not entitled to: a filing trial credited
    // $3,894 to a filer owed $0, turning an $1,085 liability into a $2,809 refund. Now the credit is
    // computed from EINs, and the only refusal left is the one case where the answer is genuinely
    // unknowable — over the cap, with an EIN missing.
    if let Some((tbl, _)) = tier.package {
        let excess_ss_max = tbl.ss_wage_base * EMPLOYEE_OASDI_RATE; // §3101(a)/§6413(c)
        let over_cap_needs_ein = |owner: Owner| -> bool {
            let mine = ri.w2s.iter().filter(|w| w.owner == owner);
            let withheld: Usd = mine.clone().map(|w| w.box4_ss_withheld).sum();
            withheld > excess_ss_max
                // ★ CANONICALIZED — a malformed EIN is as undecidable as a missing one, and a
                //   differently-spelled one is not a second employer. See `canonical_ein`.
                && mine.clone().any(|w| {
                    w.ein
                        .as_deref()
                        .and_then(crate::tax::return_1040::canonical_ein)
                        .is_none()
                })
        };
        for owner in [Owner::Taxpayer, Owner::Spouse] {
            if over_cap_needs_ein(owner) {
                return refuse(
                    RefuseReason::ExcessSsEmployerUnknown,
                    "Social Security withheld exceeds the §3101(a) cap, so whether any of it is \
                     creditable depends on whether it came from MORE THAN ONE EMPLOYER (§6413(c), \
                     Schedule 3 line 11) — and a W-2 has no EIN. Add `ein` to every W-2 for that \
                     person: one employer's over-withholding is recovered FROM THE EMPLOYER and is \
                     never claimable on the return",
                );
            }
        }
    }

    let mut deferral_tp = Usd::ZERO; // taxpayer
    let mut deferral_sp = Usd::ZERO; // spouse
    for w2 in &ri.w2s {
        // ★★★ R4 — box 13 *Statutory employee*, CHECKED. First in the loop because it is the one
        //     box that decides WHICH LINE the whole W-2 reaches: box 1 goes to Schedule C line 1,
        //     not to Form 1040 line 1a.
        if w2.box13_statutory_employee {
            return refuse(
                RefuseReason::StatutoryEmployeeW2,
                format!(
                    "the Form W-2 from {} has box 13 \"Statutory employee\" CHECKED. A statutory \
                     employee's box-1 wages are business receipts: they belong on SCHEDULE C LINE 1, \
                     with the expenses of earning them deducted against them, and NOT on Form 1040 \
                     line 1a. btctax files every transcribed W-2's box 1 on line 1a, so filing this \
                     one would put the wages on the wrong line and forgo the Schedule C deductions. \
                     File that W-2's Schedule C with a preparer",
                    if w2.employer.trim().is_empty() {
                        "(unnamed employer)"
                    } else {
                        w2.employer.trim()
                    }
                ),
            );
        }
        if w2.box8_allocated_tips > Usd::ZERO {
            return refuse(
                RefuseReason::AllocatedTips,
                "W-2 box 8 allocated tips require Form 4137",
            );
        }
        if w2.box10_dependent_care > Usd::ZERO {
            return refuse(
                RefuseReason::DependentCareBenefit,
                "W-2 box 10 dependent-care benefits require Form 2441",
            );
        }
        for entry in &w2.box12 {
            let code = entry.code.trim().to_uppercase();
            if !INERT_BOX12_CODES.contains(&code.as_str()) {
                return refuse(
                    RefuseReason::UnsupportedBox12Code(code.clone()),
                    format!("W-2 box 12 code {code} is not supported in v1"),
                );
            }
            if ELECTIVE_DEFERRAL_CODES.contains(&code.as_str()) {
                match w2.owner {
                    Owner::Taxpayer => deferral_tp += entry.amount,
                    Owner::Spouse => deferral_sp += entry.amount,
                }
            }
        }
    }
    // ★ T4/R11 — the §402(g) limit is a FullReturnParams figure: it waits for the year's package.
    if let Some((_, p)) = tier.package {
        if deferral_tp > p.elective_deferral_limit || deferral_sp > p.elective_deferral_limit {
            return refuse(
                RefuseReason::ExcessElectiveDeferral,
                "one person's elective deferrals exceed the §402(g) limit — the taxable excess (1040 line 1h) is unmodeled in v1",
            );
        }
    }

    // 1099-INT / 1099-DIV: AMT-preference bonds, special-rate gains, foreign tax over the §904(j) ceiling.
    let mut foreign_tax = Usd::ZERO;
    for int in &ri.int_1099 {
        if int.box9_private_activity_bond_amt > Usd::ZERO {
            return refuse(
                RefuseReason::PrivateActivityBondAmt,
                "1099-INT box 9 (private-activity-bond interest) is an AMT preference — out of scope",
            );
        }
        // ★★★ R4 — boxes 11, 12 and 13 (bond premium). A REDUCTION of interest income under §171,
        //     which btctax does not compute, so dropping it would OVERSTATE the tax silently.
        if int.box11_bond_premium > Usd::ZERO
            || int.box12_bond_premium_treasury > Usd::ZERO
            || int.box13_bond_premium_tax_exempt > Usd::ZERO
        {
            return refuse(
                RefuseReason::AmortizableBondPremiumNotComputed,
                "a Form 1099-INT reports AMORTIZABLE BOND PREMIUM (box 11, 12 or 13). Under §171 the \
                 premium REDUCES the interest the bond pays, and Schedule B line 1 takes it as a \
                 named subtraction — see Pub. 550, \"Amortizable bond premium\". btctax computes no \
                 part of the §171 election or that adjustment, so it would report MORE interest than \
                 you owe tax on. It refuses rather than overstate: make the line-1 adjustment by \
                 hand with a preparer",
            );
        }
        foreign_tax += int.box6_foreign_tax;
    }
    for div in &ri.div_1099 {
        // box 1b (qualified) and box 5 (§199A) are form-guaranteed SUBSETS of box 1a (ordinary). An excess
        // is a corrupt import that would give preferential / QBI treatment to income never entered in AGI
        // (a silent understatement) — fail loud, like the other inconsistent-input guards (I4).
        if div.box1b_qualified > div.box1a_ordinary {
            return refuse(
                RefuseReason::InconsistentDividendSubset("box 1b qualified dividends".to_string()),
                "a 1099-DIV box 1b (qualified dividends) exceeds its box 1a (ordinary dividends) — box 1b is \
                 a subset of box 1a; fix the import",
            );
        }
        if div.box5_section_199a > div.box1a_ordinary {
            return refuse(
                RefuseReason::InconsistentDividendSubset("box 5 §199A dividends".to_string()),
                "a 1099-DIV box 5 (§199A dividends) exceeds its box 1a (ordinary dividends) — box 5 is a \
                 subset of box 1a; fix the import",
            );
        }
        if div.box2b_unrecap_1250 > Usd::ZERO
            || div.box2c_section_1202 > Usd::ZERO
            || div.box2d_collectibles_28 > Usd::ZERO
        {
            return refuse(
                RefuseReason::UnrecapturedOrSpecialRateGain,
                "1099-DIV box 2b/2c/2d requires the Schedule D Tax Worksheet — out of scope",
            );
        }
        if div.box13_private_activity_amt > Usd::ZERO {
            return refuse(
                RefuseReason::PrivateActivityBondAmt,
                "1099-DIV box 13 (private-activity-bond dividends) is an AMT preference — out of scope",
            );
        }
        foreign_tax += div.box7_foreign_tax;
    }
    // ★★★ R4 / FR-65 — Form 1099-G box 10 *Family leave benefits*, NEW on the Rev. December 2026
    //     grid. Rev. Rul. 2025-4 makes a state paid family and medical leave program report them,
    //     and they are gross income reaching SCHEDULE 1 LINE 8z, for which btctax models no inflow.
    for g in &ri.g_1099 {
        if g.box10_family_leave_benefits > Usd::ZERO {
            return refuse(
                RefuseReason::FamilyLeaveBenefits,
                "a Form 1099-G reports FAMILY LEAVE BENEFITS in box 10 — the box the Rev. December \
                 2026 revision added so a state paid family and medical leave program can report \
                 what it paid you (Rev. Rul. 2025-4). Those benefits are income, and they reach \
                 SCHEDULE 1 LINE 8z, \"Other income. List type and amount\" — a line btctax fills \
                 from nothing, so the amount would simply vanish and understate your tax. It \
                 refuses instead. File with a preparer, who will list it on line 8z",
            );
        }
    }
    // ★ T4/R11 — the §904(j) ceiling is a FullReturnParams figure: it waits for the year's package.
    if let Some((_, p)) = tier.package {
        if foreign_tax > ftc_ceiling_for(p, ri.filing_status) {
            return refuse(
                RefuseReason::ForeignTaxOverCeiling,
                "foreign tax exceeds the §904(j) $300/$600 no-Form-1116 ceiling — Form 1116 is out of scope",
            );
        }
    }

    // Schedule 1 minimal surface: an affirmed HSA activity and any claimed IRA deduction refuse in v1.
    // (`None` — never asked — is caught by the registry's unanswered screen, P9 step 4; here we handle only
    // the affirmed `Some(true)`. `Some(false)`, a dormant holder, proceeds — un-bricking r2 C-1.)
    if ri.sch1.hsa_activity == Some(true) {
        return refuse(
            RefuseReason::HsaActivityUnsupported,
            "a Form 8889 trigger (HSA contribution, distribution, testing-period inclusion, or inheritance) \
             was affirmed — Form 8889 is out of scope for v1",
        );
    }
    if ri.sch1.ira_deduction_claimed > Usd::ZERO {
        return refuse(
            RefuseReason::IraDeductionClaimed,
            "a claimed IRA deduction needs the active-participant phase-out worksheet — unmodeled in v1",
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::return_inputs::{Box12Entry, Form1099Div, Form1099Int, W2};
    use crate::tax::return_inputs::{
        Schedule1aOvertime, Schedule1aTips, Schedule1aVehicle, ScheduleCInputs,
    };
    use crate::tax::tables::SaltLimitation;
    use crate::LedgerState;

    // A synthetic TY2024 FullReturnParams + a table with the real SS wage base for the excess-SS MAX.
    pub(super) fn params() -> FullReturnParams {
        let mut std_deduction = std::collections::BTreeMap::new();
        for s in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
        ] {
            std_deduction.insert(s, dec!(14600));
        }
        FullReturnParams {
            year: 2024,
            std_deduction,
            std_aged_blind_married: dec!(1550),
            std_aged_blind_unmarried: dec!(1950),
            dependent_std_floor: dec!(1300),
            dependent_std_earned_addon: dec!(450),
            salt: SaltLimitation::FlatCap {
                cap: dec!(10000),
                cap_mfs: dec!(5000),
            },
            kiddie_unearned_threshold: dec!(2600),
            elective_deferral_limit: dec!(23000),
            ftc_ceiling: dec!(300),
            qbi_ti_threshold_unmarried: dec!(191950),
            qbi_ti_threshold_married: dec!(383900),
            qbi_phase_in_range_unmarried: dec!(50000),
            qbi_phase_in_range_married: dec!(100000),
            student_loan_phaseout_unmarried: (dec!(80000), dec!(95000)),
            student_loan_phaseout_married: (dec!(165000), dec!(195000)),
            amt: crate::tax::tables::AmtParams {
                exemption_single_hoh: dec!(85700),
                exemption_mfj_qss: dec!(133300),
                exemption_mfs: dec!(66650),
                phaseout_start_single_hoh_mfs: dec!(609350),
                phaseout_start_mfj_qss: dec!(1218700),
                breakpoint_28pct: dec!(232600),
                breakpoint_28pct_mfs: dec!(116300),
                mfs_kicker_start: dec!(875950),
                mfs_kicker_max: dec!(66650),
                exemption_phaseout_rate: dec!(0.25),
                mfs_kicker_rate: dec!(0.25),
                rate_26: dec!(0.26),
                rate_28: dec!(0.28),
                rate_28_subtrahend: dec!(4652),
                rate_28_subtrahend_mfs: dec!(2326),
            },
        }
    }
    pub(super) fn tbl() -> TaxTable {
        crate::tax::tables::synthetic_table(2024) // ss_wage_base = 176,100 (synthetic); MAX = 10,918.20
    }
    pub(super) fn ri() -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        // ★ ANSWERED, not defaulted. Every fixture must state these — that is the whole point of D-8/P9, and
        // if `Default` supplied them these tests would be re-asserting the very guess we just removed. All
        // EIGHT always-live declarations are answered here so a computing fixture is not tripped by the
        // registry loop (§3.1 churn note); a test that wants one UNANSWERED re-blanks it explicitly.
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        ri.foreign_accounts = Some(false);
        ri.foreign_trust = Some(false);
        ri.sch1.hsa_activity = Some(false);
        ri.dual_status_alien = Some(false);
        ri.has_income_exclusion = Some(false);
        // §G-22/B11 — the scope attestation. ANSWERED here, never defaulted: `None` refuses, which is
        // the whole point, and a `Default` that supplied it would restore the silence it exists to break.
        ri.other_out_of_scope_income = Some(false);
        // Schedule D line 20 / Schedule A line 9 — the Form 4952 declaration. Always live (line 20
        // prints on every both-gains Schedule D, which liveness cannot see), so every computing
        // fixture must state it; `false` = "not filing one", the answer that needs no form v1 lacks.
        ri.filing_form_4952 = Some(false);
        // §G-28/B1b — answered whenever the fixture has a trade or business, since the SSTB question
        // is live exactly then. Set through the same path a filer would use.
        if let Some(c) = ri.schedule_c.as_mut() {
            c.is_sstb = Some(false);
        }
        // §G-9: "did not die during the tax year". No longer REQUIRED (the death gates are class-(B)
        // skippables now — see `the_death_gates_do_not_block_a_return`), but kept so that every fixture
        // below claims the age-65 box the way a real filer would, and no existing figure moves.
        ri.header.taxpayer_died_during_year = Some(false);
        // ★★★ R3 — THE DOCUMENT CENSUS, answered. Eighteen class-(A) declarations, so an unanswered
        //     one refuses exactly like every other; a fixture that left them blank would be
        //     re-asserting the silence the census exists to break. `false` on every row is the
        //     starting shape — "I received none" — and `reason()` below keeps it COHERENT as a test
        //     adds transcribed rows.
        for row in crate::tax::document_census::DocumentRow::ALL {
            ri.documents.set(*row, Some(false));
        }
        // ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR, answered. Every census row above is
        //     `Some(false)`, which is exactly what makes these three LIVE — so a fixture that left
        //     them blank would refuse on them instead of on the rule it was written to exercise.
        //     `false` is the neutral on all three: no undocumented wages, no undocumented interest
        //     or dividends, no state refund. (`itemized_prior_year` is NOT answered here: it is
        //     live only once a refund exists, and the fixtures that create one answer it.)
        ri.w2_wages_without_w2 = Some(false);
        ri.interest_or_dividends_without_1099 = Some(false);
        ri.state_refund_without_1099g = Some(false);
        ri
    }
    /// ★ R3 — screen a fixture, first making its census COHERENT with the rows it carries.
    ///
    /// Only one direction is corrected, and only one: a row answered `false` beside transcribed rows
    /// of that document becomes `true`. Every other state passes through untouched — in particular an
    /// UNANSWERED row still refuses, which is what
    /// [`every_live_unanswered_declaration_refuses_with_its_own_reason`] measures over the census.
    ///
    /// Without this, every fixture in this module that adds a W-2 after [`ri`] would report
    /// `DocumentCensusContradicted` instead of the rule it was written to exercise. The census's own
    /// three value rules are measured by [`the_document_census_refuses_each_incoherent_state`], which
    /// calls `screen_inputs` directly and so cannot be masked by this helper.
    fn reason(ri: &ReturnInputs) -> Option<RefuseReason> {
        let mut ri = ri.clone();
        for row in crate::tax::document_census::DocumentRow::ALL {
            let has_rows =
                crate::tax::document_census::declared_rows(&ri, *row).is_some_and(|n| n > 0);
            if has_rows && ri.documents.get(*row) == Some(false) {
                ri.documents.set(*row, Some(true));
            }
        }
        screen_inputs(&ri, &tbl(), &params()).map(|r| r.reason)
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // ★★★ R3 — THE DOCUMENT CENSUS. Kills (a)–(e), each measured against `screen_inputs` directly
    //     so the `reason()` coherence helper above cannot mask one.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// A fixture with EVERY declaration answered and EVERY census row answered `false` — the
    /// starting shape each kill below perturbs in exactly one way.
    fn censused() -> ReturnInputs {
        ri() // `ri()` already answers all eighteen rows `Some(false)`
    }

    fn raw(r: &ReturnInputs) -> Option<RefuseReason> {
        screen_inputs(r, &tbl(), &params()).map(|x| x.reason)
    }

    /// ★★★ **R3 kills (a), (b) and (c) — the three incoherent states, each refusing with its own
    ///     reason.** The census exists to make these three distinguishable from a correct blank, so
    ///     a test that only checked "it refuses" would not measure it.
    #[test]
    fn the_document_census_refuses_each_incoherent_state() {
        use crate::tax::document_census::DocumentRow;
        use crate::tax::return_inputs::{Owner, W2};

        // (a) UNANSWERED — a live row left `None` blocks, through the registry loop.
        let mut a = censused();
        a.documents.int_1099 = None;
        assert_eq!(
            raw(&a),
            Some(RefuseReason::DocumentCensusUnanswered {
                kind: DocumentRow::Int1099
            }),
            "a live census row left unanswered must refuse: \"none\" and \"nobody asked\" are the \
             same blank on the page and are not the same testimony"
        );

        // (b) DECLARED, NOT TRANSCRIBED — `Some(true)` on a countable row with zero rows.
        let mut b = censused();
        b.documents.w2 = Some(true);
        assert!(b.w2s.is_empty(), "the fixture starts with no W-2");
        assert_eq!(
            raw(&b),
            Some(RefuseReason::DocumentDeclaredNotTranscribed {
                kind: DocumentRow::W2
            }),
            "a declared document with nothing transcribed is exactly \"nothing ever populated it\""
        );

        // (c) CONTRADICTED — `Some(false)` beside a transcribed row.
        let mut c = censused();
        c.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(1000),
            box3_ss_wages: dec!(1000),
            box5_medicare_wages: dec!(1000),
            ..Default::default()
        }];
        assert_eq!(c.documents.w2, Some(false));
        assert_eq!(
            raw(&c),
            Some(RefuseReason::DocumentCensusContradicted {
                kind: DocumentRow::W2
            }),
            "a \"no\" beside a transcribed row of that document must refuse rather than be picked \
             between"
        );

        // …and the coherent pair does NOT refuse — without this the three reds above could all be
        // one blanket refusal.
        let mut ok = c.clone();
        ok.documents.w2 = Some(true);
        assert_eq!(raw(&ok), None, "a coherent census must not refuse");
    }

    /// ★★★ **R3 kill (d) — every refusing row's `Some(true)` carries §2.2's own exit sentence,
    ///     VERBATIM, and names the document the filer answered about.**
    ///
    /// Both halves matter. Without the verbatim check a refusal could be a paraphrase that drops
    /// the exit; without the designation check the two §2.2 sentences that name no form at all
    /// (SSA-1099, W-2G) would leave the filer unable to tell WHICH answer produced the refusal.
    #[test]
    fn every_unsupported_census_row_refuses_with_its_own_exit_sentence() {
        use crate::tax::document_census::DocumentRow;
        let mut refused = 0usize;
        for row in DocumentRow::ALL {
            let mut r = censused();
            r.documents.set(*row, Some(true));
            let Some(exit) = row.exit_sentence() else {
                // W2 (transcribable) and the two scalar-shadowed rows have no exit.
                continue;
            };
            let refusal = screen_inputs(&r, &tbl(), &params())
                .unwrap_or_else(|| panic!("{row:?} = Yes must refuse — its exit is: {exit}"));
            assert_eq!(
                refusal.reason,
                RefuseReason::DocumentTypeUnsupported { kind: *row },
                "{row:?} = Yes must refuse as UNSUPPORTED"
            );
            assert!(
                refusal.detail.contains(exit),
                "{row:?}'s refusal must carry §2.2's sentence VERBATIM.\n  want: {exit}\n  got:  {}",
                refusal.detail
            );
            assert!(
                refusal.detail.contains(row.designation()),
                "{row:?}'s refusal must name the document the filer answered about ({}): {}",
                row.designation(),
                refusal.detail
            );
            refused += 1;
        }
        assert_eq!(
            refused, 11,
            "§2.2's eleven excluded families refuse on Yes — a shrinking count means a family \
             stopped being announced"
        );
    }

    /// ★★★ **THE FOUR 1099 ROWS DECLARE LIKE THE W-2 ROW — AND ONLY THREE OF THEM DEMAND ROWS.**
    ///
    /// The declaration, the contradiction and the block are the W-2's rules for all four (the
    /// pre-review D1 fold, which replaced `the_four_t5_rows_refuse_naming_their_task` — that test
    /// pinned the opposite and was watched going RED on the change). What the T3 seam review then
    /// found is that the *fourth* rule, "a declared document must be transcribed", is not the W-2's
    /// rule for two of them:
    ///
    /// - **1099-B** — the form's own instruction makes the Schedule D line 1a/8a summary optional
    ///   (*"if you choose to report all these transactions on Form 8949, leave this line blank"*),
    ///   and btctax's own population is exactly that case: the dispositions are in the ledger and
    ///   print per transaction. Zero rows is the CORRECT return.
    /// - **1099-G** — `Form1099G` has no box 2, so the commonest 1099-G of all (a prior-year state
    ///   refund) cannot be transcribed truthfully at all. **T5** adds the box and flips it.
    ///
    /// So the truth table below is over TWO predicates, and the second is what rule (2) is gated on.
    #[test]
    fn the_four_1099_rows_declare_like_the_w2_row_and_only_three_demand_rows() {
        use crate::tax::document_census::{requires_transcription, DocumentRow};
        use crate::tax::return_inputs::{Form1099B, Form1099Div, Form1099G, Form1099Int};

        // One row of each document, added to a censused fixture by the same path `income import`
        // writes: the `Vec` on `ReturnInputs`.
        let with_one_row = |row: DocumentRow| -> ReturnInputs {
            let mut r = censused();
            match row {
                DocumentRow::Int1099 => r.int_1099 = vec![Form1099Int::default()],
                DocumentRow::Div1099 => r.div_1099 = vec![Form1099Div::default()],
                DocumentRow::B1099 => r.b_1099 = vec![Form1099B::default()],
                DocumentRow::G1099 => r.g_1099 = vec![Form1099G::default()],
                other => panic!("{other:?} is not one of the four"),
            }
            r
        };

        for row in [
            DocumentRow::Int1099,
            DocumentRow::Div1099,
            DocumentRow::B1099,
            DocumentRow::G1099,
        ] {
            // ── (1) `Some(true)` WITH a row PASSES. The whole point of the fold. ────────────────
            let mut ok = with_one_row(row);
            ok.documents.set(row, Some(true));
            assert_eq!(
                raw(&ok),
                None,
                "{row:?} = Yes with one transcribed row must FILE — the filer entered it through \
                 `income import`, and a census that refused it would be demanding false testimony"
            );

            // ── (2) `Some(true)` with ZERO rows — the ONE rule that splits, on
            //        `requires_transcription`. Where rows are demanded it refuses and NAMES the way
            //        in; where the form itself offers a blank, a truthful Yes FILES. ─────────────
            let mut declared = censused();
            declared.documents.set(row, Some(true));
            let got = screen_inputs(&declared, &tbl(), &params());
            if requires_transcription(row) {
                let r = got.unwrap_or_else(|| {
                    panic!("{row:?} declared with nothing transcribed must refuse")
                });
                assert_eq!(
                    r.reason,
                    RefuseReason::DocumentDeclaredNotTranscribed { kind: row },
                    "{row:?}: a declared document with no transcription is UNANSWERED, not \
                     UNSUPPORTED"
                );
                assert!(
                    r.detail.contains("income import"),
                    "{row:?}'s refusal must name the route in — a refusal with no exit is a brick \
                     with better prose: {}",
                    r.detail
                );
                // ★★★ **T5 CHANGED WHAT THIS ASSERTS, and the change is the deliverable.** It used
                //     to demand the detail name the TASK that would build the screen (`T5`), which
                //     was honest while the only way in was a TOML table. T5 built the screens, so
                //     the route must now name the FORM SECTION — and a route still pointing at a
                //     task would mean the section it promised does not exist.
                assert!(
                    r.detail.contains("tax-inputs form"),
                    "{row:?}'s refusal must name the FORM SECTION now that T5 has built it: {}",
                    r.detail
                );
                assert!(
                    !r.detail.contains("task T5"),
                    "{row:?}'s refusal must not still promise a task that has landed: {}",
                    r.detail
                );
            } else {
                assert_eq!(
                    got.map(|r| r.reason),
                    None,
                    "{row:?} does not require transcription — a truthful Yes with zero rows is a \
                     CORRECT return and must file"
                );
            }

            // ── (3) `Some(false)` beside a transcribed row refuses the contradiction. ──────────
            let mut no = with_one_row(row);
            no.documents.set(row, Some(false));
            assert_eq!(
                raw(&no),
                Some(RefuseReason::DocumentCensusContradicted { kind: row }),
                "{row:?} = No beside a transcribed row must refuse, exactly as the W-2 row does"
            );

            // ── (4) `None` blocks, through the registry loop. ──────────────────────────────────
            let mut unanswered = with_one_row(row);
            unanswered.documents.set(row, None);
            assert_eq!(
                raw(&unanswered),
                Some(RefuseReason::DocumentCensusUnanswered { kind: row }),
                "{row:?} unanswered must block"
            );

            // ── (5) …and none of the four is an EXCLUDED family any more. ─────────────────────
            assert_eq!(
                row.exit_sentence(),
                None,
                "{row:?} must carry no §2.2 exit — btctax can hold its rows today"
            );
        }
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // ★★★ R3 / R4 / T5 — THE DOCUMENT-LESS INCOME DOOR, AND THE BOX DECISIONS THAT REFUSE
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ **R3 KILL (f), VERBATIM.** *"Each paired question is live exactly when its row is
    ///     `Some(false)` and blocks there — `w2 = Some(false)` with `w2_wages_without_w2 = None`
    ///     refuses; `w2 = Some(true)` never asks it; a `Yes` on the wage question refuses naming
    ///     line 1a, a `Yes` on the refund question refuses naming the State and Local Income Tax
    ///     Refund Worksheet, and a `Yes` on the interest/dividend question with no
    ///     `schedule_b_filer_records` row refuses while one row passes."*
    ///
    /// ★ The BICONDITIONAL is what makes it a pairing rather than three extra always-live
    ///   questions: a filer holding the document is never asked whether they hold income without it.
    #[test]
    fn each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there() {
        use crate::tax::document_census::DocumentRow;
        use crate::tax::questions::{question_is_live, QuestionId};

        // ── (f1) LIVE exactly on `Some(false)`, for each of the three pairings. ─────────────────
        for (row, q) in [
            (DocumentRow::W2, QuestionId::WagesWithoutW2Question),
            (
                DocumentRow::Int1099,
                QuestionId::InterestOrDividendsWithout1099,
            ),
            (DocumentRow::G1099, QuestionId::StateRefundWithout1099g),
        ] {
            let mut yes = censused();
            yes.documents.set(row, Some(true));
            // ★ The 1099-INT/DIV pairing shares ONE question and EITHER row's `No` opens it, so the
            //   `Some(true)` probe must close both.
            if row == DocumentRow::Int1099 {
                yes.documents.set(DocumentRow::Div1099, Some(true));
            }
            assert!(
                !question_is_live(q, &yes),
                "{row:?} = Yes: the filer HAS the document, so {q:?} must not be asked"
            );
            let no = censused(); // `censused()` answers every census row `Some(false)`
            assert!(
                question_is_live(q, &no),
                "{row:?} = No: {q:?} must be live — a census `No` never closes income the \
                 instructions say to report WITHOUT the document"
            );
            // …and left UNANSWERED it BLOCKS with its own reason. `censused()` pre-answers the
            // three door questions (so every other fixture in this module exercises its own rule),
            // so the probe blanks exactly the one under test.
            let entry = crate::tax::questions::FORM_QUESTIONS
                .iter()
                .find(|e| e.id == q)
                .expect("a registry question");
            let mut blank = censused();
            match q {
                QuestionId::WagesWithoutW2Question => blank.w2_wages_without_w2 = None,
                QuestionId::InterestOrDividendsWithout1099 => {
                    blank.interest_or_dividends_without_1099 = None;
                }
                QuestionId::StateRefundWithout1099g => blank.state_refund_without_1099g = None,
                other => panic!("{other:?} is not one of the three paired questions"),
            }
            assert!((entry.get)(&blank).is_none(), "the probe starts unanswered");
            assert_eq!(
                raw(&blank).as_ref(),
                Some(&entry.unanswered),
                "a live {q:?} left None must block with its own unanswered reason"
            );
            // Answering it neutrally clears exactly that reason.
            (entry.set)(&mut blank, false);
            assert_ne!(raw(&blank).as_ref(), Some(&entry.unanswered));
        }

        // ── (f2) `Yes` on the WAGE question refuses, naming Form 1040 line 1a. ──────────────────
        let mut wages = censused();
        wages.w2_wages_without_w2 = Some(true);
        let r = screen_inputs(&wages, &tbl(), &params()).expect("undocumented wages refuse");
        assert_eq!(r.reason, RefuseReason::WagesWithoutW2);
        assert!(
            r.detail.contains("LINE 1a"),
            "the refusal must name the line the earnings belong on: {}",
            r.detail
        );
        assert!(
            r.detail.contains("Even if you don't get a Form W-2"),
            "…in the instructions' own words: {}",
            r.detail
        );

        // ── (f3) `Yes` on the REFUND question takes the worksheet exit — through §111(a), which is
        //         what decides whether any of it is income at all. ────────────────────────────────
        let mut refund = censused();
        refund.state_refund_without_1099g = Some(true);
        assert_eq!(
            raw(&refund),
            Some(RefuseReason::ItemizedPriorYearUnanswered),
            "a Yes makes the §111(a) gate live, and an unanswered class-(A) declaration blocks"
        );
        let mut itemized = refund.clone();
        itemized.itemized_prior_year = Some(true);
        let r = screen_inputs(&itemized, &tbl(), &params()).expect("a taxable refund refuses");
        assert_eq!(
            r.reason,
            RefuseReason::StateAndLocalRefundWorksheetNotComputed
        );
        assert!(
            r.detail
                .contains("STATE AND LOCAL INCOME TAX REFUND WORKSHEET"),
            "the refusal must name the worksheet: {}",
            r.detail
        );
        // …and a filer who did NOT itemize owes nothing on it: §111(a), line 1 blank BY DECISION.
        let mut not_itemized = refund.clone();
        not_itemized.itemized_prior_year = Some(false);
        assert_eq!(
            raw(&not_itemized),
            None,
            "no tax benefit ⇒ no income ⇒ the return files with Schedule 1 line 1 blank"
        );

        // ── (f4) `Yes` on the INTEREST question with no row refuses; ONE row passes. ────────────
        let mut door = censused();
        door.interest_or_dividends_without_1099 = Some(true);
        let r =
            screen_inputs(&door, &tbl(), &params()).expect("a declared record with none refuses");
        assert_eq!(r.reason, RefuseReason::FilerRecordsDeclaredNotTranscribed);
        assert!(
            r.detail.contains("schedule_b_filer_records"),
            "the refusal must name the way in: {}",
            r.detail
        );
        let mut with_row = door.clone();
        with_row.schedule_b_filer_records = vec![crate::tax::return_inputs::ScheduleBRecord {
            payer_name: "Neighbour, on a seller-financed mortgage".into(),
            payer_ssn: "000-00-0001".into(),
            payer_address: "1 Example St".into(),
            amount: dec!(1200),
            kind: crate::tax::return_inputs::ScheduleBRecordKind::Interest,
        }];
        assert_eq!(
            raw(&with_row),
            None,
            "one filer's-records row is the whole remedy — the return must file"
        );
    }

    /// ★★★ **R3 KILL (g).** *"`itemized_prior_year` is live on a return with **no** 1099-G row when
    ///     the refund question is `Some(true)`."*
    ///
    /// This is R3/I1's whole reason: *a gate may not ride on a row that might not exist.* Had the
    /// gate stayed a `Form1099G` field, this filer — who received the refund and no 1099-G — could
    /// never have been asked, and Schedule 1 line 1 would have been blank by accident.
    #[test]
    fn the_prior_year_itemize_gate_is_live_with_no_1099g_row_at_all() {
        use crate::tax::questions::{question_is_live, QuestionId};
        let mut r = censused();
        assert!(r.g_1099.is_empty(), "the probe's premise: no 1099-G row");
        assert!(
            !question_is_live(QuestionId::ItemizedPriorYear, &r),
            "with no refund anywhere the gate must be silent"
        );
        r.state_refund_without_1099g = Some(true);
        assert!(
            question_is_live(QuestionId::ItemizedPriorYear, &r),
            "★ THE KILL: the §111(a) gate must reach a filer whose refund arrived with no document"
        );
        assert!(r.g_1099.is_empty(), "…and it did so with no row to hang on");
    }

    /// ★★★ **R4 — W-2 BOX 13 CHECKED REFUSES, NAMING SCHEDULE C LINE 1.**
    ///
    /// A checked box 13 sends box 1 to Schedule C, not to Form 1040 line 1a. Before T5 the `W2`
    /// struct had no box 13 at all, so a statutory employee's wages filed on the wrong line with
    /// nothing on the return to notice it.
    #[test]
    fn a_checked_statutory_employee_box_refuses_naming_schedule_c_line_1() {
        use crate::tax::document_census::DocumentRow;
        let mut r = censused();
        r.documents.set(DocumentRow::W2, Some(true));
        r.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(50000),
            box13_statutory_employee: true,
            ..Default::default()
        }];
        let got = screen_inputs(&r, &tbl(), &params()).expect("a checked box 13 refuses");
        assert_eq!(got.reason, RefuseReason::StatutoryEmployeeW2);
        assert!(
            got.detail.contains("SCHEDULE C LINE 1"),
            "the refusal must name the line the wages belong on: {}",
            got.detail
        );
        assert!(
            got.detail.contains("ACME"),
            "…and WHICH W-2, since a household may hold several: {}",
            got.detail
        );
        // ★ Unchecked, the same return files — or the guard is "refuse every W-2".
        r.w2s[0].box13_statutory_employee = false;
        assert_eq!(raw(&r), None);
    }

    /// ★★★ **R4 — 1099-INT BOXES 11, 12 AND 13 REFUSE, NAMING PUB. 550.**
    ///
    /// §171 amortizable bond premium REDUCES the interest reported, through a named Schedule B
    /// line-1 adjustment btctax does not compute. Dropping it would OVERSTATE the tax silently, so
    /// refusing is both the conservative and the honest direction.
    ///
    /// ★ All THREE boxes, one at a time: a guard written for box 11 alone would leave the two
    ///   Treasury and tax-exempt premiums dropping silently, and the census entries for 12 and 13
    ///   would be describing a refusal that never fires.
    #[test]
    fn any_bond_premium_box_refuses_naming_pub_550() {
        use crate::tax::document_census::DocumentRow;
        for plant in [
            "box11_bond_premium",
            "box12_bond_premium_treasury",
            "box13_bond_premium_tax_exempt",
        ] {
            let mut r = censused();
            r.documents.set(DocumentRow::Int1099, Some(true));
            let mut row = Form1099Int {
                payer: "First Bank".into(),
                box1_interest: dec!(500),
                ..Default::default()
            };
            match plant {
                "box11_bond_premium" => row.box11_bond_premium = dec!(40),
                "box12_bond_premium_treasury" => row.box12_bond_premium_treasury = dec!(40),
                _ => row.box13_bond_premium_tax_exempt = dec!(40),
            }
            r.int_1099 = vec![row];
            let got = screen_inputs(&r, &tbl(), &params())
                .unwrap_or_else(|| panic!("{plant} > 0 must refuse"));
            assert_eq!(
                got.reason,
                RefuseReason::AmortizableBondPremiumNotComputed,
                "{plant}"
            );
            assert!(
                got.detail.contains("Pub. 550"),
                "{plant}: the refusal must name the publication that explains it: {}",
                got.detail
            );
            assert!(
                got.detail.contains("overstate"),
                "{plant}: …and the DIRECTION, which is why it refuses rather than drops: {}",
                got.detail
            );
        }
        // ★ Every premium box zero ⇒ the same return files.
        let mut clean = censused();
        clean.documents.set(DocumentRow::Int1099, Some(true));
        clean.int_1099 = vec![Form1099Int {
            payer: "First Bank".into(),
            box1_interest: dec!(500),
            ..Default::default()
        }];
        assert_eq!(raw(&clean), None);
    }

    /// ★★★ **THE TWO FILERS THE ONE-PREDICATE RULE REFUSED (T3 seam review I2 / I3).**
    ///
    /// Both answer the census TRUTHFULLY and both have the correct number of rows — zero — and both
    /// were refused `DocumentDeclaredNotTranscribed` before the split. This is the review's own
    /// probe, committed: the two shapes are ordinary populations, not corner cases, and the second
    /// is the commonest 1099-G there is.
    ///
    /// ★ The mutation that reds it is one word: `requires_transcription(B1099) => true`.
    #[test]
    fn a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file() {
        use crate::tax::document_census::DocumentRow;

        // ── P1. The crypto filer — btctax's OWN population. Their exchange's Form 1099-B covers
        //        dispositions that are already in the ledger and print per transaction on Form 8949
        //        and Schedule D, so the summary line is blank BY THE FORM'S OWN INSTRUCTION. There
        //        is no `[[b_1099]]` row, and entering one would double-count every gain
        //        (`form_1099b_gains` is added into `capital_net`).
        let mut p1 = censused();
        p1.documents.set(DocumentRow::B1099, Some(true));
        assert!(
            p1.b_1099.is_empty(),
            "the probe's premise: zero summary rows"
        );
        assert_eq!(
            raw(&p1),
            None,
            "a filer who received a Form 1099-B and reports every transaction on Form 8949 answers \
             YES truthfully and transcribes NOTHING — the form prints that option, and refusing it \
             would leave them a choice between false testimony and a double-counted return"
        );

        // ── P2. The itemizer's prior-year state refund — box 2 alone, the commonest 1099-G.
        //
        //     ★★★ **T5 CLOSED THIS HALF OF THE SPLIT, which is why it is asserted the other way
        //         round now.** The filer was excused from transcribing because `Form1099G` HAD NO
        //         BOX 2 — the excuse was the absence of a field, never a property of the document.
        //         T5 added `box2_state_refund`, so the row exists, `requires_transcription(G1099)`
        //         is `true`, and the honest outcome is: TRANSCRIBE THE ROW, then §111(a) decides.
        //
        //     ★ The B1099 half above is untouched and still files with zero rows, because ITS
        //       excuse is the form's own printed instruction rather than a missing field. Keeping
        //       both probes in one test is what shows the two were never the same reason.
        let mut p2 = censused();
        p2.documents.set(DocumentRow::G1099, Some(true));
        assert!(p2.g_1099.is_empty(), "the probe's premise: zero rows");
        assert_eq!(
            raw(&p2),
            Some(RefuseReason::DocumentDeclaredNotTranscribed {
                kind: DocumentRow::G1099
            }),
            "since T5 the 1099-G HAS a box-2 field, so a declared one with nothing transcribed is \
             once again exactly \"nothing ever populated it\""
        );

        // ★★★ …and the row the filer then enters — **whose only amount is box 2** — PASSES the
        //     census and reaches the §111(a) gate, which is the whole point of adding the field.
        let mut p2b = censused();
        p2b.documents.set(DocumentRow::G1099, Some(true));
        p2b.g_1099 = vec![crate::tax::return_inputs::Form1099G {
            payer: "State of Example".into(),
            box2_state_refund: dec!(900),
            ..Default::default()
        }];
        // Unanswered, the §111(a) gate BLOCKS (it is live: box 2 > 0).
        assert_eq!(
            raw(&p2b),
            Some(RefuseReason::ItemizedPriorYearUnanswered),
            "a transcribed box-2 refund makes the prior-year-itemized gate live, and an unanswered \
             class-(A) declaration blocks"
        );
        // Answered NO — §111(a): no tax benefit, so none of it is income and the return FILES with
        // Schedule 1 line 1 blank BY DECISION.
        let mut no = p2b.clone();
        no.itemized_prior_year = Some(false);
        assert_eq!(
            raw(&no),
            None,
            "a filer who did not itemize in the year they paid the tax owes nothing on the refund \
             — Schedule 1 line 1 is correctly blank and the return must file"
        );
        // Answered YES — the worksheet btctax does not compute.
        let mut yes = p2b.clone();
        yes.itemized_prior_year = Some(true);
        let r = screen_inputs(&yes, &tbl(), &params()).expect("an itemizer's refund refuses");
        assert_eq!(
            r.reason,
            RefuseReason::StateAndLocalRefundWorksheetNotComputed
        );
        assert!(
            r.detail
                .contains("STATE AND LOCAL INCOME TAX REFUND WORKSHEET"),
            "the refusal must name the worksheet it cannot compute: {}",
            r.detail
        );

        // ── …and the refusals the split did NOT relax. ────────────────────────────────────────
        let mut no_beside_a_row = censused();
        no_beside_a_row.b_1099 = vec![crate::tax::return_inputs::Form1099B::default()];
        no_beside_a_row
            .documents
            .set(DocumentRow::B1099, Some(false));
        assert_eq!(
            raw(&no_beside_a_row),
            Some(RefuseReason::DocumentCensusContradicted {
                kind: DocumentRow::B1099
            }),
            "the CONTRADICTION rule runs on all five `Vec`-bearing rows — `declared_rows` is what \
             was split away from the demand, not switched off"
        );
        let mut declared_w2 = censused();
        declared_w2.documents.set(DocumentRow::W2, Some(true));
        assert_eq!(
            raw(&declared_w2),
            Some(RefuseReason::DocumentDeclaredNotTranscribed {
                kind: DocumentRow::W2
            }),
            "a W-2 reaches Form 1040 line 1a ONLY through `w2s`, so a declared one with no row is \
             still exactly \"nothing ever populated it\""
        );
    }

    /// ★★★ **THE TWO PREDICATES, over every one of the eighteen rows, as one table.**
    ///
    /// `declared_rows` is a fact about the return; `requires_transcription` is a demand on the
    /// filer. They are `Some`/`true` on different sets, and a checker that could not tell the two
    /// apart is what refused the two truthful filers above. The invariant that binds them: a row may
    /// only DEMAND rows if it has somewhere to count them.
    #[test]
    fn the_rows_invariant_is_two_predicates_and_a_demand_needs_somewhere_to_count() {
        use crate::tax::document_census::{declared_rows, requires_transcription, DocumentRow};
        let ri = censused();
        let countable: Vec<DocumentRow> = DocumentRow::ALL
            .iter()
            .copied()
            .filter(|r| declared_rows(&ri, *r).is_some())
            .collect();
        assert_eq!(
            countable,
            vec![
                DocumentRow::W2,
                DocumentRow::Int1099,
                DocumentRow::Div1099,
                DocumentRow::B1099,
                DocumentRow::G1099,
                // ★ T5 — the 1098-E gained a `Vec` when `Form1098E` replaced the
                //   `sch1.student_loan_interest_paid` scalar.
                DocumentRow::Form1098e,
            ],
            "the six `Vec`-bearing kinds, and only those, have a row count"
        );
        let demanding: Vec<DocumentRow> = DocumentRow::ALL
            .iter()
            .copied()
            .filter(|r| requires_transcription(*r))
            .collect();
        assert_eq!(
            demanding,
            vec![
                DocumentRow::W2,
                DocumentRow::Int1099,
                DocumentRow::Div1099,
                // ★★★ **T5 PUT THE 1099-G BACK IN, and the reason it was out is the reason it is
                //     back**: it was excused because `Form1099G` HAD NO BOX 2, so the commonest
                //     1099-G had nowhere to be transcribed. T5 added `box2_state_refund`, so the
                //     excuse expired with the field.
                DocumentRow::G1099,
                // ★ And the 1098-E, whose rows replaced `sch1.student_loan_interest_paid`: nothing
                //   else carries student-loan interest onto the return any more.
                DocumentRow::Form1098e,
            ],
            "★ 1099-B is the only supported row still OUT, and its excuse is the FORM'S OWN printed \
             blank (Schedule D line 1a/8a is a summary option, and the ledger is the crypto filer's \
             1099-B) — never a missing field. Adding it back must be a deliberate edit with a \
             screen behind it."
        );
        for row in DocumentRow::ALL {
            assert!(
                !requires_transcription(*row) || declared_rows(&ri, *row).is_some(),
                "{row:?} demands transcription with nowhere to count it — the demand would be \
                 unsatisfiable"
            );
        }
    }

    /// ★★ **The route a refusal names must not be a route that files a wrong return.** The 1099-B
    /// and 1099-G entry routes are latent today (neither row demands transcription), and T5 makes
    /// the 1099-G's live — so they are read HERE rather than left as prose nobody checks.
    #[test]
    fn the_1099b_and_1099g_routes_name_the_blank_and_the_scalar_not_a_double_count() {
        use crate::tax::document_census::DocumentRow;
        let b = DocumentRow::B1099
            .entry_route()
            .expect("1099-B has a route");
        assert!(
            b.contains("Form 8949") && b.contains("leave this line blank"),
            "the 1099-B route must name the FORM'S OWN option — reporting on Form 8949 — before it \
             mentions a summary row: {b}"
        );
        assert!(
            b.contains("count the same gains twice"),
            "…and must say what entering one beside ledger dispositions would do: {b}"
        );
        // ★★★ **T5 CLOSED THE 1099-G'S SPLIT ROUTE.** It used to send a box-2-only filer to the
        //     `sch1.state_refund_taxable` scalar and name T5 as the task that would build the field.
        //     T5 built it, so the route now names ONE place for the whole document and says which
        //     line each box reaches — and it must NOT still send anyone to the scalar, which would
        //     be a second, silent entry path for the same figure.
        let g = DocumentRow::G1099
            .entry_route()
            .expect("1099-G has a route");
        assert!(
            g.contains("box 2") && g.contains("Schedule 1 line 1"),
            "the 1099-G route must name where BOX 2 reaches the return: {g}"
        );
        assert!(
            g.contains("box 1") && g.contains("Schedule 1 line 7"),
            "…and where box 1 reaches it: {g}"
        );
        assert!(
            !g.contains("sch1.state_refund_taxable"),
            "…and must no longer send a box-2 filer to the scalar, which T5 replaced with a field: \
             {g}"
        );
    }

    /// ★★ **The scalar-shadowed row is NOT LIVE: answering it asks nothing and refuses nothing** —
    ///     on any fixture, at any value.
    ///
    /// Its amount is collected today by a scalar, so a `No` would contradict a figure already
    /// entered and a `Yes` would demand a section that does not exist. The kill runs BOTH values,
    /// because a liveness bug that let only one through would otherwise pass.
    ///
    /// ★★★ **T5 TOOK `form_1098e` OUT OF THIS SET, and the other half of the pair is asserted here
    ///     so the removal cannot be silent**: the 1098-E row is now LIVE, so an unanswered one
    ///     BLOCKS. A test that merely stopped listing it would have proved nothing.
    #[test]
    fn the_scalar_shadowed_census_row_is_not_live_and_never_refuses() {
        use crate::tax::document_census::DocumentRow;
        for answer in [None, Some(true), Some(false)] {
            let mut r = censused();
            r.documents.set(DocumentRow::Form1098, answer);
            assert_eq!(
                raw(&r),
                None,
                "Form1098 = {answer:?} must neither block nor refuse until T9 replaces its scalar"
            );
        }
        // ★ THE OTHER HALF: the 1098-E row opened at T5, so an unanswered one blocks.
        let mut opened = censused();
        opened.documents.set(DocumentRow::Form1098e, None);
        assert_eq!(
            raw(&opened),
            Some(RefuseReason::DocumentCensusUnanswered {
                kind: DocumentRow::Form1098e
            }),
            "the 1098-E row is LIVE since T5 — an unanswered one must block, or the row is a \
             question nobody is ever asked"
        );
    }

    /// ★★★ The ABSOLUTE screen — the one that can see the §63(e) election. Two of the phase-2
    /// refusals moved here from `screen_inputs` (phase-2 review R2/R5a, Critical): both condition a
    /// SCHEDULE A DEDUCTION, and `screen_inputs` sees only inputs, so sitting there they refused
    /// filers who take the STANDARD deduction and never print the line at all. Returns the election
    /// alongside the reason so every fixture must state which side of it it is on — a fixture that
    /// silently flipped to standard would otherwise "pass" by not being screened.
    fn absolute_reason(r: &ReturnInputs) -> (bool, Option<RefuseReason>) {
        let st = LedgerState::default();
        let ar = assemble_absolute(r, &st, &params(), &tbl(), 2024);
        (
            ar.deduction_is_itemized,
            crate::tax::return_1040::screen_absolute(
                r,
                &ar,
                &params(),
                &st,
                2024,
                crate::forms::InformationReturnRegime::NONE,
            )
            .map(|x| x.reason),
        )
    }

    /// ★★★ **R10.3's KILL AT THE SCREEN — an answer given under earlier words does not stand.**
    ///
    /// The three states have to be told apart, and only the middle one refuses:
    ///
    /// 1. no record at all — the pre-log corpus; the leaf's own value decides (must NOT refuse);
    /// 2. a record whose `prompt_hash` disagrees with the registry's current words — REFUSES as
    ///    UNANSWERED, naming the changed wording and the exit;
    /// 3. a record hashing the words the registry asks today — stands.
    ///
    /// ★ State 1 is the half that would be easy to get wrong in the fail-CLOSED direction and would
    ///   still look "safe": treating an absent record as unanswered refuses every return in the
    ///   corpus, and asserts a provenance nobody ever collected.
    #[test]
    fn an_answer_hashed_against_earlier_words_refuses_and_a_missing_record_does_not() {
        use crate::tax::provenance::{prompt_hash, record_answer, AnswerKey, AnswerState};
        use crate::tax::questions::{QuestionId, FORM_QUESTIONS};

        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::ForeignTrust)
            .unwrap();

        // (1) The answered leaf with NO record — the whole existing corpus. Must compute.
        let base = ri();
        assert_eq!(base.foreign_trust, Some(false));
        assert!(
            base.answer_log.is_empty(),
            "the baseline must carry no record, or this test is not testing state (1)"
        );
        assert_eq!(reason(&base), None, "an ABSENT record must not refuse");

        // (2) A record hashed against words nobody asks any more.
        let mut stale = base.clone();
        record_answer(
            &mut stale,
            AnswerKey::Question(QuestionId::ForeignTrust),
            "Do you have a foreign trust? (an earlier draft of this sentence)",
            time::macros::date!(2026 - 09 - 01),
            AnswerState::Given,
        );
        assert_eq!(
            reason(&stale),
            Some(q.unanswered.clone()),
            "a record hashed against EARLIER words must refuse as UNANSWERED"
        );
        let detail = screen_inputs(&stale, &tbl(), &params()).unwrap().detail;
        assert!(
            detail.contains(crate::tax::provenance::WORDING_CHANGED_REASON),
            "the refusal must give R12's own reason: {detail}"
        );
        assert!(
            detail.contains("income answer"),
            "and its exit — a refusal with no exit is a brick with better prose: {detail}"
        );

        // (3) Re-answered under the CURRENT words: it stands again.
        let mut fresh = base.clone();
        record_answer(
            &mut fresh,
            AnswerKey::Question(QuestionId::ForeignTrust),
            q.prompt,
            time::macros::date!(2026 - 09 - 01),
            AnswerState::Given,
        );
        assert_eq!(
            fresh.answer_log[&AnswerKey::Question(QuestionId::ForeignTrust)].prompt_hash,
            prompt_hash(q.prompt)
        );
        assert_eq!(
            reason(&fresh),
            None,
            "a current-wording record must not refuse"
        );
    }

    /// ★ **D-8 — and this guard shipped, once, with no test at all.**
    ///
    /// The flag used to be a bare `bool` with `#[serde(default)]`, so "never asked" and "answered No" were
    /// the same value and the engine silently chose the answer that UNDERSTATES tax. Deleting the fix and
    /// re-running the suite passed 1715/1715 — every fixture simply answers the question now, so nothing
    /// was asserting the refusal FIRES. These four tests are that assertion.
    #[test]
    fn an_unanswered_dependent_flag_refuses() {
        let mut r = ri();
        r.header.can_be_claimed_as_dependent_taxpayer = None; // as a pre-D-8 vault loads
        assert_eq!(reason(&r), Some(RefuseReason::DependentStatusUnanswered));
    }

    /// Both ANSWERS are accepted — the refusal is about silence, not about the content of the answer.
    #[test]
    fn an_answered_dependent_flag_does_not_refuse() {
        let mut r = ri();
        r.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        assert_eq!(reason(&r), None);
        r.header.can_be_claimed_as_dependent_taxpayer = Some(true);
        assert_ne!(
            reason(&r),
            Some(RefuseReason::DependentStatusUnanswered),
            "a claimable filer ANSWERED — it must not be treated as unanswered"
        );
    }

    // ── P9 step 4: the registry derivations ──────────────────────────────────────────────────────

    use crate::tax::questions::{QuestionId, FORM_QUESTIONS};

    /// A Single return with EVERY always-live declaration answered "no". The baseline the property test
    /// blanks one question at a time from. (Single ⇒ DependentSpouse and MfsSpouseItemizes are not live;
    /// no `schedule_a` ⇒ the mortgage question is not live.)
    fn fully_answered() -> ReturnInputs {
        let mut r = ri(); // answers DependentTaxpayer
        r.foreign_accounts = Some(false);
        r.foreign_trust = Some(false);
        r.sch1.hsa_activity = Some(false);
        r.dual_status_alien = Some(false);
        r.filing_form_4952 = Some(false);
        r
    }

    /// A minimal return set up so `id` is LIVE, with NOTHING answered yet (every question `None`). The
    /// property test answers all questions EXCEPT the target, so the target's `None` is the sole defect.
    fn scenario_for(id: QuestionId) -> ReturnInputs {
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ReturnInputs {
            // ★★ §G-15 — a POPULATED fixture must state its year. `Default` gives `0` ("not
            // stated"), and a year-scoped question is correctly NOT live without one — so a
            // yearless scenario could never exercise `HasIncomeExclusion` and this invariant would
            // silently stop covering it. 2025 is the year in which every registry question is live.
            tax_year: 2025,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        match id {
            QuestionId::DependentSpouse => r.filing_status = FilingStatus::Mfj, // live with no spouse Person (P8a I1)
            QuestionId::MfsSpouseItemizes => r.filing_status = FilingStatus::Mfs,
            // ★ R10.4 / T4b — live ONLY on a year the opener made, which is what `opened_from` says.
            QuestionId::FilingStatusConfirmed => r.opened_from = Some(2024),
            QuestionId::MortgageAllUsedToBuyBuildImprove
            | QuestionId::AmtQualifiedDwelling
            | QuestionId::MortgageWithinDebtLimit => {
                r.schedule_a = Some(ScheduleAInputs {
                    mortgage_interest_1098: dec!(9000),
                    ..Default::default()
                });
            }
            // ★ All three carryforward-conditioned declarations share ONE liveness predicate
            //   (`questions::carryforward_in_present`), so they share one scenario.
            QuestionId::AmtCarryoverSameAsRegular
            | QuestionId::CarryoverIncludesSpousesJointLoss
            | QuestionId::ExcludedCanceledDebt => {
                r.capital_loss_carryforward_in = crate::tax::types::Carryforward {
                    short: dec!(1000),
                    long: Usd::ZERO,
                };
            }
            // ★★★ R3/I1 / T5 — the RETURN-LEVEL §111(a) gate. Made live from the DOCUMENT side (a
            //     transcribed 1099-G box 2), not from the sibling question's answer, so the scenario
            //     does not depend on another registry entry's value. The census row is answered
            //     `Some(true)` here because the row exists: leaving it to the loop's neutral
            //     (`false`) would fire `DocumentCensusContradicted` first and mask the target.
            QuestionId::ItemizedPriorYear => {
                r.g_1099 = vec![crate::tax::return_inputs::Form1099G {
                    payer: "State of Example".into(),
                    box2_state_refund: dec!(900),
                    ..Default::default()
                }];
                r.documents
                    .set(crate::tax::document_census::DocumentRow::G1099, Some(true));
            }
            QuestionId::AmtDepreciationSameAsRegular => {
                // A nonzero FLAT expense total is the whole liveness condition — btctax cannot see
                // whether Part II line 13 inside it is $0 or $200,000. See `amt_depreciation_question_live`.
                r.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
                    expenses: dec!(5000),
                    ..Default::default()
                });
            }
            _ => {}
        }
        r
    }

    /// ★ THE PER-QUESTION PROPERTY TEST (§3.5). For each registry entry: build a return where it is LIVE and
    /// blank, assert `screen_inputs` refuses with THAT entry's reason; then answer it and assert that reason
    /// no longer fires. Anchored to the registry, but the completeness anchor (questions.rs) is what stops a
    /// dropped entry from silently dropping its own scenario (r1 I-4).
    #[test]
    fn every_live_unanswered_declaration_refuses_with_its_own_reason() {
        for q in FORM_QUESTIONS {
            let mut r = scenario_for(q.id); // nothing answered yet
                                            // Answer every OTHER live question, leaving q blank (None, from Default).
                                            // ★ `is_none()` is not tidiness: a question whose LIVENESS depends on another question's
                                            //   NON-NEUTRAL answer would otherwise be switched off by this very loop, and its scenario
                                            //   could never be exercised. Only fill in what the scenario left blank. (The case that
                                            //   exposed it — the Schedule B 7a FBAR sub-question, live only at 7a = Yes while 7a's
                                            //   neutral is `false` — is now a SKIPPABLE, so no current entry exercises this guard; the
                                            //   rule stands because the next such question would hit the same wall silently.)
            for other in FORM_QUESTIONS {
                if other.id != q.id && (other.live)(&r) && (other.get)(&r).is_none() {
                    (other.set)(&mut r, other.neutral);
                }
            }
            // ★★ R3 — ONE CENSUS ROW IS DELIBERATELY NOT LIVE YET (Form 1098 → T9). Its amount is
            //    collected today by a scalar, so a `No` on the row would contradict a figure already
            //    entered and a `Yes` would demand a transcription section that does not exist. A
            //    never-live entry cannot be exercised by this property — but the skip is DERIVED
            //    from the census's own liveness predicate, not from a name list, so the moment T9
            //    flips `row_is_live` the entry re-enters this loop with no edit here. ★ That is
            //    exactly what happened to `Form1098e` at T5: it left this skip on its own.
            //    Anything else that is not live in its own scenario is a scenario bug and still
            //    fails.
            if !(q.live)(&r) {
                let row = crate::tax::document_census::row_of_question(q.id);
                assert!(
                    row.is_some_and(|row| !crate::tax::document_census::row_is_live(&r, row)),
                    "{:?} is not live in its own scenario and is not a census row awaiting its \
                     screen (T9)",
                    q.id
                );
                continue;
            }
            assert!((q.live)(&r), "{:?} must be live in its own scenario", q.id);
            assert!((q.get)(&r).is_none(), "{:?} must start blank", q.id);
            assert_eq!(
                reason(&r),
                Some(q.unanswered.clone()),
                "blank {:?} must refuse with its own unanswered reason",
                q.id
            );
            // ★ §3.5 mandates "answer it (n AND y)". Both answers remove ITS unanswered reason — a
            // value-refusal on a different axis (e.g. `Some(true)` ⇒ unsupported for four of the eight) may
            // still fire, so assert the SPECIFIC unanswered reason is gone, not that all is well (r3 NIT-1).
            for answer in [false, true] {
                (q.set)(&mut r, answer);
                assert_ne!(
                    reason(&r),
                    Some(q.unanswered.clone()),
                    "{:?} answered {answer} must no longer fire its unanswered reason",
                    q.id
                );
            }
        }
    }

    /// ★ P9 §2.7 / §3.5 (r5 I-2) — a mixed-use-mortgage filer who answers "no" truthfully is NOT bricked:
    /// the return COMPUTES, with Schedule A line 8a = $0, the line-8 box CHECKED, and
    /// `MixedUseMortgageNotAllocated` firing — under BOTH `Auto` (where the zeroed 8a lets the standard
    /// deduction win) AND `ForceItemize`. r3 refused outright (bricking the standard-wins filer, r3 I-2);
    /// r4's screen-layer refusal could not see the itemize decision it fired on (r4 I-2). There is NO
    /// mortgage refusal left — the answer zeroes the line and checks the box instead — and this proves it.
    #[test]
    fn mixed_use_mortgage_filer_computes_under_both_elections() {
        use crate::state::LedgerState;
        use crate::tax::advisories::{advisories_for, Advisory};
        use crate::tax::return_1040::assemble_absolute;
        use crate::tax::return_inputs::{ItemizeElection, Owner, ScheduleAInputs};

        let mut base = ri(); // Single, all always-live declarations answered
        base.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            box1_wages: dec!(120000),
            ..Default::default()
        }];
        base.schedule_a = Some(ScheduleAInputs {
            salt_real_estate: dec!(5000), // itemized ≈ $5,000 (< $14,600 std) once the mixed-use 8a is zeroed
            mortgage_interest_1098: dec!(12000),
            mortgage_all_used_to_buy_build_improve: Some(false),
            // Reporting 1098 interest also makes Form 6251 line 3's AMT-qualified-dwelling question
            // live (i6251 p.8). Answered AMT-neutral here so this test keeps testing the MIXED-USE
            // question rather than tripping on the new one.
            mortgage_dwelling_is_amt_qualified: Some(true),
            // …and, since §163(h)(3)(B), the acquisition-debt-ceiling question too. Same reason:
            // neutral, so the MIXED-USE branch is what this test still exercises.
            mortgage_within_debt_limit: Some(true),
            ..Default::default()
        });

        for (election, expect_itemized) in [
            (ItemizeElection::Auto, false), // zeroed 8a ⇒ the standard deduction wins
            (ItemizeElection::ForceItemize, true), // §63(e) forces the tiny Schedule A
        ] {
            let mut r = base.clone();
            r.itemize_election = election;

            // No brick: the screen does not refuse a truthfully-answered mixed-use return.
            assert_eq!(reason(&r), None, "{election:?}: must not refuse");

            // …and it COMPUTES, with 8a zeroed and the box checked, under either deduction.
            let ar = assemble_absolute(&r, &LedgerState::default(), &params(), &tbl(), 2024);
            assert_eq!(ar.deduction_is_itemized, expect_itemized, "{election:?}");
            let a = ar.schedule_a.as_ref().expect("Schedule A parts computed");
            assert_eq!(a.mortgage_8a, Usd::ZERO, "{election:?}: 8a zeroed");
            assert!(a.mortgage_mixed_use_box, "{election:?}: line-8 box checked");

            // …and the owner-mandate advisory fires, naming the full 1098 interest as the ceiling, with the
            // branch matching the deduction actually taken.
            let advs = advisories_for(&r, &LedgerState::default(), &ar, &params(), 2024);
            assert!(
                advs.contains(&Advisory::MixedUseMortgageNotAllocated {
                    forgone_interest: dec!(12000),
                    itemized: expect_itemized,
                }),
                "{election:?}: the advisory must fire with the ceiling and the right branch: {advs:?}"
            );
        }
    }

    /// ★ §2.9 — THE CIRCULAR-LIVENESS BUG, in shipped code. A filer with $100 of interest and an unanswered
    /// foreign-account question must REFUSE. Under the shipped `schedule_b_files` (which reads
    /// `foreign_accounts` itself) the return computes clean and silently omits Schedule B — the FBAR/FinCEN
    /// disclosure. This test is red on the pre-P9 boundary; the always-live registry entry turns it green.
    #[test]
    fn a_foreign_account_question_is_live_even_below_the_schedule_b_threshold() {
        let mut r = fully_answered();
        r.int_1099 = vec![crate::tax::return_inputs::Form1099Int {
            payer: "Bank".into(),
            box1_interest: dec!(100), // WELL below the $1,500 Schedule B threshold
            ..Default::default()
        }];
        r.foreign_accounts = None; // never asked
        assert_eq!(
            reason(&r),
            Some(RefuseReason::ScheduleBPart3Unanswered),
            "an unanswered foreign-account question must refuse regardless of the Schedule B threshold (§2.9)"
        );
    }

    /// ★ P8a I1 — an MFJ return with NO spouse `Person` record still owes the joint dependent-spouse box.
    /// The shipped scope (`spouse.is_some()`) missed it; the registry liveness `Mfj || spouse.is_some()`
    /// catches it.
    #[test]
    fn mfj_with_no_spouse_record_still_requires_the_dependent_spouse_answer() {
        let mut r = fully_answered();
        r.filing_status = FilingStatus::Mfj;
        r.header.spouse = None; // no spouse Person on the return
        r.header.can_be_claimed_as_dependent_spouse = None; // and the joint box is unanswered
        assert_eq!(
            reason(&r),
            Some(RefuseReason::DependentSpouseStatusUnanswered),
            "MFJ owes the spouse-dependent box even with no spouse Person (P8a I1)"
        );
    }

    /// The spouse question is only a question when there IS a spouse. Asking it of a Single filer would be
    /// an unanswerable refusal — a return you could never file.
    #[test]
    fn an_unanswered_spouse_flag_refuses_only_when_a_spouse_is_on_the_return() {
        let mut single = ri();
        single.header.can_be_claimed_as_dependent_spouse = None;
        assert_eq!(reason(&single), None, "no spouse ⇒ no spouse question");

        let mut joint = ri();
        joint.filing_status = FilingStatus::Mfj;
        joint.header.spouse = Some(crate::tax::return_inputs::Person {
            first_name: "Pat".into(),
            last_name: "Doe".into(),
            ssn: "987654321".into(),
            ..Default::default()
        });
        joint.header.can_be_claimed_as_dependent_spouse = None;
        assert_eq!(
            reason(&joint),
            Some(RefuseReason::DependentSpouseStatusUnanswered)
        );
    }

    /// ★ The refusal is what LETS the compute layer project the tri-state down to a `bool`. If it ever
    /// stops firing before compute, `standard_deduction` silently grants the full basic std to a filer who
    /// should get the §63(c)(5) floor — an understatement. This test pins the two together: the flag must
    /// still be unanswerable-and-refused at the screen that gates compute.
    #[test]
    fn the_unanswered_refusal_is_what_guards_the_63c5_floor() {
        let mut unanswered = ri();
        unanswered.header.can_be_claimed_as_dependent_taxpayer = None;
        assert!(
            screen_inputs(&unanswered, &tbl(), &params()).is_some(),
            "compute must never see an unanswered flag — it would fall through to the basic std"
        );

        // ★ And if compute ever DOES see an unknown flag, it must err toward the SMALLER deduction. This
        // is the assertion `== Some(true)` could not make: `unwrap_or(false)` is indistinguishable from
        // `== Some(true)` for a bool, so a style rule alone tests nothing. Pinning the None branch to the
        // dependent floor makes the safe direction a fact the suite can check.
        let p = params();
        let mut unknown = ri();
        unknown.header.can_be_claimed_as_dependent_taxpayer = None;
        let mut claimable = ri();
        claimable.header.can_be_claimed_as_dependent_taxpayer = Some(true);
        let mut not_claimable = ri();
        not_claimable.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        let sd =
            |r: &ReturnInputs| crate::tax::return_1040::standard_deduction(r, &p, 2024, Usd::ZERO);
        assert_eq!(
            sd(&unknown),
            sd(&claimable),
            "an UNKNOWN flag must take the §63(c)(5) floor — the direction that overstates tax"
        );
        assert!(
            sd(&unknown) < sd(&not_claimable),
            "...and that floor must really be the smaller deduction, or 'fail-closed' means nothing"
        );

        // And the two answers really do compute DIFFERENT deductions, so the question is load-bearing.
        let mut dep = ri();
        dep.header.can_be_claimed_as_dependent_taxpayer = Some(true);
        let mut not_dep = ri();
        not_dep.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        let earned = Usd::ZERO;
        assert_ne!(
            crate::tax::return_1040::standard_deduction(&dep, &p, 2024, earned),
            crate::tax::return_1040::standard_deduction(&not_dep, &p, 2024, earned),
            "if these were equal the flag would not matter and this whole refusal would be pointless"
        );
    }

    /// ★ **Fable P7 r3 I1.** A Schedule C the return does not NAME cannot be filed.
    ///
    /// This guard shipped in the r2 fold with ZERO tests: the reviewer deleted it and all 1708 tests
    /// still passed. It is not decoration, and it is not merely belt-and-braces behind the Form 8995
    /// filler's own fail-closed. It is the **only** guard on **Schedule C line A** — because a business
    /// whose net profit is at or below the §6017 $400 SE floor produces no QBI, hence no Form 8995 at
    /// all, so the filler's check never runs, and `schedule_c.rs` writes line A only when it is
    /// non-empty. Without this, that filer files a Schedule C whose "Principal business or profession"
    /// is BLANK.
    ///
    /// `business_description` is `#[serde(default)]`, so an imported TOML that simply omits the key
    /// yields `""` — this is not a hypothetical.
    #[test]
    fn a_schedule_c_with_no_business_description_refuses() {
        let mut r = ri();
        r.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: String::new(), // as an import omitting the key would give
            is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
            ..Default::default()
        });
        assert_eq!(
            reason(&r),
            Some(RefuseReason::ScheduleCNoBusinessDescription),
            "a Schedule C with no name must refuse — line A and Form 8995 row 1i(a) both require it"
        );

        // Whitespace is not a name. This pins the `trim()`, which a naive `is_empty()` would miss.
        let mut ws = ri();
        ws.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "   ".into(),
            is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
            ..Default::default()
        });
        assert_eq!(
            reason(&ws),
            Some(RefuseReason::ScheduleCNoBusinessDescription),
            "three spaces are not the name of a trade or business"
        );

        // The negative leg: a real name does NOT refuse. Without this the test would pass on a screen
        // that refuses every Schedule C ever.
        let mut ok = ri();
        ok.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "Bitcoin mining".into(),
            is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
            ..Default::default()
        });
        assert_eq!(reason(&ok), None, "a NAMED business must file");
    }

    /// ★★ A MALFORMED SSN DOES NOT BLOCK THE COMPUTATION — and the boundary that does refuse it is
    /// asserted in the same breath, because deleting a gate is only safe if you can name the one behind it.
    ///
    /// It used to refuse at compute time. That made a single typo in an identity field block `report`,
    /// `optimize`, `what-if` and the whole TUI — **none of which read an SSN** — and it was strictly
    /// harsher than an EMPTY SSN, which has always been let through (see the sibling test). There is no
    /// number on the return an unparseable SSN can make wrong.
    ///
    /// The identity boundary is the FILABLE PACKET, and it is unchanged: `ReturnHeader::build` returns
    /// `HeaderError::Ssn(..)` for `Missing`, `NotDigits` AND `WrongLength`, so a typo still cannot reach a
    /// printed comb cell. Each of the three malformed shapes is exercised on both sides.
    #[test]
    fn a_malformed_ssn_computes_but_the_packet_still_refuses_it() {
        use crate::tax::packet::{ReturnHeader, SsnError};
        // (label, the malformed SSN, the SsnError the packet boundary must raise)
        let shapes = [
            ("five digits", "12345", SsnError::WrongLength(5)),
            ("a non-digit", "123-45-678X", SsnError::NotDigits('X')),
            ("ten digits", "1234567890", SsnError::WrongLength(10)),
        ];
        for (label, ssn, expected) in shapes {
            // ── the taxpayer's own SSN ──
            let mut r = ri();
            r.header.taxpayer.ssn = ssn.into();
            assert_eq!(
                reason(&r),
                None,
                "{label}: a typo must NOT block the report — nothing reads an SSN"
            );
            assert_eq!(
                ReturnHeader::build(&r, 2024).unwrap_err(),
                crate::tax::packet::HeaderError::Ssn(expected),
                "{label}: …but the FILABLE PACKET must still refuse it"
            );

            // ── a spouse's, on a joint return ──
            let mut r = ri();
            r.filing_status = FilingStatus::Mfj;
            r.header.can_be_claimed_as_dependent_spouse = Some(false);
            r.header.spouse = Some(crate::tax::return_inputs::Person {
                ssn: ssn.into(),
                ..Default::default()
            });
            r.header.spouse_died_during_year = Some(false);
            assert_eq!(reason(&r), None, "{label}: spouse — report computes");
            assert!(
                matches!(
                    ReturnHeader::build(&r, 2024),
                    Err(crate::tax::packet::HeaderError::Ssn(_))
                ),
                "{label}: spouse — packet refuses"
            );

            // ── a dependent's ──
            let mut r = ri();
            r.header
                .dependents
                .push(crate::tax::return_inputs::Dependent {
                    name: "Sam Doe".into(),
                    ssn: ssn.into(),
                    ..Default::default()
                });
            assert_eq!(reason(&r), None, "{label}: dependent — report computes");
            assert!(
                matches!(
                    ReturnHeader::build(&r, 2024),
                    Err(crate::tax::packet::HeaderError::Ssn(_))
                ),
                "{label}: dependent — packet refuses"
            );
        }
    }

    /// ★ An **uncaptured** SSN is not the same as a malformed one. The tax math does not read an SSN, so
    /// a household that has not entered its PII yet still gets a REPORT — it is only the filable PACKET
    /// that refuses (`ReturnHeader::build` → `SsnError::Missing`). Refusing the computation too would
    /// block the very report a filer uses to decide whether to file at all, and would buy no correctness:
    /// there is no number on the return that an absent SSN could make wrong.
    #[test]
    fn an_uncaptured_ssn_does_not_block_the_report() {
        let mut r = ri();
        r.w2s.push(W2 {
            box1_wages: dec!(80000),
            ..Default::default()
        });
        assert_eq!(r.header.taxpayer.ssn, "", "the fixture captured no PII");
        assert_eq!(reason(&r), None, "…and the report still computes");
    }

    #[test]
    fn clean_return_is_not_refused() {
        let mut r = ri();
        r.w2s.push(W2 {
            box1_wages: dec!(80000),
            box12: vec![Box12Entry {
                code: "DD".into(),
                amount: dec!(18000),
            }],
            ..Default::default()
        });
        r.div_1099.push(Form1099Div {
            box1a_ordinary: dec!(3000),
            box7_foreign_tax: dec!(120), // ≤ $300 → OK
            ..Default::default()
        });
        // $3,000 dividends files Schedule B, so Part III (7a/8) must be answered to stay clean.
        r.foreign_accounts = Some(false);
        r.foreign_trust = Some(false);
        assert_eq!(reason(&r), None);
    }

    #[test]
    fn box12_code_k_refuses_but_allowlist_ok() {
        let mut r = ri();
        r.w2s.push(W2 {
            box12: vec![Box12Entry {
                code: "K".into(),
                amount: dec!(500),
            }],
            ..Default::default()
        });
        assert_eq!(
            reason(&r),
            Some(RefuseReason::UnsupportedBox12Code("K".into()))
        );
        // A 401(k) household's code D is inert.
        let mut ok = ri();
        ok.w2s.push(W2 {
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(20000),
            }],
            ..Default::default()
        });
        assert_eq!(reason(&ok), None);
    }

    #[test]
    fn excess_402g_deferral_is_per_person() {
        // Same owner (both taxpayer): $15k + $10k = $25k > $23k → refuse.
        let mut r = ri();
        r.w2s.push(W2 {
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(15000),
            }],
            ..Default::default()
        });
        r.w2s.push(W2 {
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(10000),
            }],
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::ExcessElectiveDeferral));
        // MFJ dual-earner: $15k taxpayer + $15k spouse — each under $23k → NO refuse (review I1).
        let mut ok = ri();
        ok.filing_status = FilingStatus::Mfj;
        ok.header.can_be_claimed_as_dependent_spouse = Some(false); // MFJ makes the spouse box live (P8a I1)
        ok.w2s.push(W2 {
            owner: Owner::Taxpayer,
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(15000),
            }],
            ..Default::default()
        });
        ok.w2s.push(W2 {
            owner: Owner::Spouse,
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(15000),
            }],
            ..Default::default()
        });
        assert_eq!(reason(&ok), None);
    }

    #[test]
    fn box8_box10_refuse() {
        let mut a = ri();
        a.w2s.push(W2 {
            box8_allocated_tips: dec!(500),
            ..Default::default()
        });
        assert_eq!(reason(&a), Some(RefuseReason::AllocatedTips));
        let mut b = ri();
        b.w2s.push(W2 {
            box10_dependent_care: dec!(5000),
            ..Default::default()
        });
        assert_eq!(reason(&b), Some(RefuseReason::DependentCareBenefit));
    }

    /// ★★★ §G-22/B11 — BOTH legs of the scope attestation, and the `Some(true)` leg shipped with NO
    /// test at all: r8 deleted its whole refusal block and 2559/2559 stayed green. The unanswered leg
    /// was covered incidentally (every fixture answers it), which is exactly the kind of accidental
    /// coverage B1 exists to distinguish from a kill.
    #[test]
    fn the_scope_attestation_refuses_unanswered_and_affirmed_alike() {
        // `None` — never asked. Silence is not testimony that there is no rental income.
        let mut unanswered = ri();
        unanswered.other_out_of_scope_income = None;
        assert_eq!(
            reason(&unanswered),
            Some(RefuseReason::OtherIncomeUnanswered)
        );

        // `Some(true)` — the filer AFFIRMED income v1 cannot model. Filing anyway would omit §61
        // income, so this refuses rather than emitting a packet that is silently short.
        let mut affirmed = ri();
        affirmed.other_out_of_scope_income = Some(true);
        assert_eq!(reason(&affirmed), Some(RefuseReason::OtherIncomeOutOfScope));

        // `Some(false)` — answered, and the return proceeds. Without this the rule could refuse
        // everything, which catches nothing.
        let mut answered = ri();
        answered.other_out_of_scope_income = Some(false);
        assert_eq!(reason(&answered), None);
    }

    #[test]
    fn excess_ss_refuses_only_when_employer_identity_is_unknown() {
        // ★★★ Over the §3101(a) cap with NO EIN: the credit turns on "more than one employer" and we
        //     cannot tell. Refuse and collect it — guessing either way is a real figure on a signed
        //     return, and guessing "yes" UNDERSTATES tax.
        let mut unknown = ri();
        unknown.w2s.push(W2 {
            box4_ss_withheld: dec!(11000),
            ..Default::default()
        });
        assert_eq!(
            reason(&unknown),
            Some(RefuseReason::ExcessSsEmployerUnknown)
        );

        // ★★ …but a SINGLE employer over-withholding no longer refuses at all. i1040gi says "you can't
        //    claim the excess on your return. The employer should adjust the tax for you" — NOT "you
        //    can't file". The return is complete and correct with a $0 credit. This is the shape of the
        //    TaxCalcBench vector `mfj-schedule-2-multiple-w2-excess-social-security-tax`, which btctax
        //    previously could not file at all.
        let mut single = ri();
        single.w2s.push(W2 {
            box4_ss_withheld: dec!(11000),
            ein: Some("11-1111111".into()),
            ..Default::default()
        });
        assert_eq!(
            reason(&single),
            None,
            "a single employer's over-withholding is not creditable, but the return still FILES"
        );

        // Two employers, identity stated → no refusal; the credit is computed.
        let mut two = ri();
        for e in ["11-1111111", "22-2222222"] {
            two.w2s.push(W2 {
                box4_ss_withheld: dec!(6000),
                ein: Some(e.into()),
                ..Default::default()
            });
        }
        assert_eq!(reason(&two), None);

        // Under the cap → employer identity never matters, and no EIN is demanded.
        let mut under = ri();
        under.w2s.push(W2 {
            box4_ss_withheld: dec!(1000),
            ..Default::default()
        });
        assert_eq!(
            reason(&under),
            None,
            "an EIN is only required when it decides something"
        );
    }

    #[test]
    fn amt_preference_and_special_gains_refuse() {
        let mut a = ri();
        a.int_1099.push(Form1099Int {
            box9_private_activity_bond_amt: dec!(10),
            ..Default::default()
        });
        assert_eq!(reason(&a), Some(RefuseReason::PrivateActivityBondAmt));
        let mut b = ri();
        b.div_1099.push(Form1099Div {
            box2d_collectibles_28: dec!(50),
            ..Default::default()
        });
        assert_eq!(
            reason(&b),
            Some(RefuseReason::UnrecapturedOrSpecialRateGain)
        );
    }

    #[test]
    fn dividend_subset_inconsistency_refuses() {
        // Part III answered so the Schedule-B trigger doesn't mask the subset check.
        let answered = || {
            let mut r = ReturnInputs {
                filing_status: FilingStatus::Single,
                foreign_accounts: Some(false),
                foreign_trust: Some(false),
                ..Default::default()
            };
            // ...and the D-8/P9 always-live declarations, which `answered()` is named for.
            r.header.can_be_claimed_as_dependent_taxpayer = Some(false);
            r.sch1.hsa_activity = Some(false);
            r.dual_status_alien = Some(false);
            r.has_income_exclusion = Some(false);
            r.other_out_of_scope_income = Some(false); // §G-22/B11
            r.filing_form_4952 = Some(false); // Schedule D line 20 / Schedule A line 9
            r.header.taxpayer_died_during_year = Some(false); // §G-9
                                                              // ★ R3 — and the eighteen document-census rows, for the same reason.
            for row in crate::tax::document_census::DocumentRow::ALL {
                r.documents.set(*row, Some(false));
            }
            // ★ R3 / T5 — every census row `false` makes the three document-less income questions
            //   live, so they are answered here too (see `ri`).
            r.w2_wages_without_w2 = Some(false);
            r.interest_or_dividends_without_1099 = Some(false);
            r.state_refund_without_1099g = Some(false);
            r
        };
        // I4: box 1b (qualified) > box 1a (ordinary) on a form ⇒ refuse (phantom preferential income).
        let mut a = answered();
        a.div_1099.push(Form1099Div {
            box1a_ordinary: dec!(10000),
            box1b_qualified: dec!(15000),
            ..Default::default()
        });
        assert_eq!(
            reason(&a),
            Some(RefuseReason::InconsistentDividendSubset(
                "box 1b qualified dividends".into()
            ))
        );
        // box 5 (§199A) > box 1a ⇒ refuse (phantom QBI base).
        let mut b = answered();
        b.div_1099.push(Form1099Div {
            box1a_ordinary: dec!(5000),
            box5_section_199a: dec!(8000),
            ..Default::default()
        });
        assert_eq!(
            reason(&b),
            Some(RefuseReason::InconsistentDividendSubset(
                "box 5 §199A dividends".into()
            ))
        );
        // Fully-qualified and all-REIT (box 1b == box 5 == box 1a) is legitimate → no refusal.
        let mut ok = answered();
        ok.div_1099.push(Form1099Div {
            box1a_ordinary: dec!(10000),
            box1b_qualified: dec!(10000),
            box5_section_199a: dec!(10000),
            ..Default::default()
        });
        assert_eq!(reason(&ok), None);
    }

    #[test]
    fn foreign_tax_over_ceiling_refuses() {
        // Single: $301 > $300 ceiling.
        let mut r = ri();
        r.div_1099.push(Form1099Div {
            box7_foreign_tax: dec!(301),
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::ForeignTaxOverCeiling));
        // MFJ ceiling is doubled ($600): $301 is fine.
        let mut mfj = r.clone();
        mfj.filing_status = FilingStatus::Mfj;
        mfj.header.can_be_claimed_as_dependent_spouse = Some(false); // MFJ makes the spouse box live (P8a I1)
        assert_eq!(reason(&mfj), None);
        // QSS is NOT a joint return — ceiling stays $300, so $301 refuses (review I2).
        let mut qss = r.clone();
        qss.filing_status = FilingStatus::Qss;
        assert_eq!(reason(&qss), Some(RefuseReason::ForeignTaxOverCeiling));
    }

    #[test]
    fn negative_amount_refuses_before_any_threshold_offset() {
        // R2-I1 PoC-A: a +$500 foreign tax (over the $300 ceiling → must refuse) plus a −$250 sign typo
        // must NOT net to $250 ≤ $300 and pass — the negative screen refuses FIRST.
        let mut r = ri();
        r.div_1099.push(Form1099Div {
            box7_foreign_tax: dec!(500),
            ..Default::default()
        });
        r.int_1099.push(Form1099Int {
            box6_foreign_tax: dec!(-250),
            ..Default::default()
        });
        assert_eq!(
            reason(&r),
            Some(RefuseReason::NegativeAmount(
                "1099-INT box 6 foreign tax".into()
            ))
        );
        // Same shape for a negative elective deferral (the old M4 vector) and a plain negative wage.
        let mut d = ri();
        d.w2s.push(W2 {
            box12: vec![
                Box12Entry {
                    code: "D".into(),
                    amount: dec!(30000),
                },
                Box12Entry {
                    code: "D".into(),
                    amount: dec!(-10000),
                },
            ],
            ..Default::default()
        });
        assert_eq!(
            reason(&d),
            Some(RefuseReason::NegativeAmount("W-2 box 12 amount".into()))
        );
        let mut w = ri();
        w.w2s.push(W2 {
            box1_wages: dec!(-1),
            ..Default::default()
        });
        assert_eq!(
            reason(&w),
            Some(RefuseReason::NegativeAmount("W-2 box 1 wages".into()))
        );
    }

    #[test]
    fn spouse_owned_item_on_non_joint_return_refuses() {
        // R2-I2 PoC-B: Single filer, a second W-2 mislabeled owner="spouse" would split one person's
        // $30k deferrals into two ≤$23k buckets. Refuse the mislabel before it can evade the §402(g) cap.
        let mut single = ri(); // filing_status = Single
        single.w2s.push(W2 {
            owner: Owner::Taxpayer,
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(15000),
            }],
            ..Default::default()
        });
        single.w2s.push(W2 {
            owner: Owner::Spouse,
            box12: vec![Box12Entry {
                code: "D".into(),
                amount: dec!(15000),
            }],
            ..Default::default()
        });
        assert_eq!(
            reason(&single),
            Some(RefuseReason::SpouseOwnerWithoutJointReturn)
        );
        // A spouse-owned Schedule C on a non-joint return also refuses.
        let mut hoh = ri();
        hoh.filing_status = FilingStatus::HoH;
        hoh.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            owner: Owner::Spouse,
            is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
            ..Default::default()
        });
        assert_eq!(
            reason(&hoh),
            Some(RefuseReason::SpouseOwnerWithoutJointReturn)
        );
        // The SAME split on a joint return is legitimate (two earners) → no spouse-owner refusal.
        let mut mfj = single.clone();
        mfj.filing_status = FilingStatus::Mfj;
        mfj.header.can_be_claimed_as_dependent_spouse = Some(false); // MFJ makes the spouse box live (P8a I1)
        assert_eq!(reason(&mfj), None);
    }

    #[test]
    fn schedule_b_part3_unanswered_refuses() {
        // Above the $1,500 threshold, an unanswered Part III still refuses (the below-threshold case — the
        // §2.9 bug — is covered separately). `ri()` now answers the foreign questions, so re-blank 7a.
        let mut r = ri();
        r.int_1099.push(Form1099Int {
            box1_interest: dec!(2000),
            ..Default::default()
        });
        r.foreign_accounts = None; // re-blank line 7a
        assert_eq!(reason(&r), Some(RefuseReason::ScheduleBPart3Unanswered));
        // Answer 7a; now line 8 (foreign trust) unanswered → still fail-loud (registry covers both).
        r.foreign_accounts = Some(false);
        r.foreign_trust = None;
        assert_eq!(reason(&r), Some(RefuseReason::ScheduleBPart3Unanswered));
        // Both answered → no refusal.
        r.foreign_trust = Some(false);
        assert_eq!(reason(&r), None);
    }

    /// ★★ Schedule B 7a's unnumbered FBAR sub-question is **class (B)** — it must NOT refuse.
    ///
    /// It was briefly a class-(A) declaration, and that was the error the refusal review corrected:
    /// a refusal is justified only when proceeding would put a wrong number or fabricated testimony
    /// on the return, or silently expose the filer to a penalty. This box fails all three — no
    /// figure reads it, a blank is no testimony, and the penalty the form's Caution warns of is for
    /// not FILING FinCEN Form 114, an obligation this box neither creates nor removes. So a return
    /// with 7a = Yes and the sub-question BLANK must compute clean, and be advised, not refused.
    #[test]
    fn the_fbar_sub_question_does_not_refuse_a_return() {
        let mut r = ri();
        r.foreign_accounts = Some(true); // 7a Yes ⇒ the sub-question is asked
        r.foreign_country_names = "Portugal".to_string(); // 7b, else its own value-refusal fires
        r.fbar_filing_required = None; // …and skipped
        assert_eq!(
            reason(&r),
            None,
            "a skipped FBAR sub-question is LAWFUL silence — it may not block the return"
        );
        // Answering it either way is equally fine.
        for answered in [Some(true), Some(false)] {
            r.fbar_filing_required = answered;
            assert_eq!(reason(&r), None, "answered {answered:?}");
        }
        // ★ And it is not in the mandatory registry at all — a re-added FORM_QUESTIONS entry would
        // re-introduce the refusal silently, since the registry loop screens every live entry.
        assert!(
            !FORM_QUESTIONS
                .iter()
                .any(|q| (q.get)(&r) == r.fbar_filing_required
                    && format!("{:?}", q.id).contains("Fbar")),
            "the FBAR sub-question is a SKIPPABLE, never a mandatory declaration"
        );
        assert!(
            btctax_skippables().any(|s| format!("{s:?}").contains("Fbar")),
            "…and it IS in the skippable registry, so it is still ASKED"
        );
    }

    /// ★★★ THE §G-9 DEATH GATES DO NOT BLOCK A RETURN. `TaxpayerDiedDuringYear` was a class-(A)
    /// declaration with `live: |_| true`, so an unanswered one refused **every return btctax could
    /// compute** — the single largest usability cost in the registry.
    ///
    /// It bought nothing. `is_aged`'s `(None, None)` arm already returns `false`, so silence FORGOES
    /// the §63(f) age-65 addition rather than granting it on an unresolved carve-out: the refusal was
    /// redundant with a fail-safe sitting directly beneath it, and the direction of the residual error
    /// is OVERSTATEMENT, which §3.4 permits and advises on. This test is the whole claim: a return
    /// that answers nothing about death computes, and the box it would have claimed is NOT granted.
    #[test]
    fn the_death_gates_do_not_block_a_return() {
        use crate::tax::packet::ReturnHeader;
        let mut r = ri();
        r.header.taxpayer_died_during_year = None; // re-blank what the fixture answers
        r.w2s.push(W2 {
            box1_wages: dec!(80000),
            ..Default::default()
        });
        assert_eq!(
            reason(&r),
            None,
            "an unanswered death gate must NOT refuse — silence is lawful here"
        );

        // …and a filer old enough to qualify does NOT get the box while it is unanswered.
        r.header.taxpayer.date_of_birth = Some(time::macros::date!(1955 - 03 - 02)); // 65+ in 2024
        r.header.taxpayer.ssn = "123456789".into();
        r.header.taxpayer.first_name = "John".into();
        r.header.taxpayer.last_name = "Doe".into();
        assert_eq!(reason(&r), None, "still computes with a DOB on file");
        let h = ReturnHeader::build(&r, 2024).unwrap();
        assert!(
            !h.aged_blind.taxpayer_aged,
            "★ the age-65 box is FORGONE while the death carve-out is unresolved — never granted. \
             Flipping this to `true` restores the understatement §G-9 fixed."
        );

        // Answering it "no" claims the box, which is what makes the forfeit above a real cost.
        r.header.taxpayer_died_during_year = Some(false);
        assert!(
            ReturnHeader::build(&r, 2024)
                .unwrap()
                .aged_blind
                .taxpayer_aged,
            "answered ⇒ the box is claimed"
        );
    }

    /// ★ The SPOUSE death gate is MFJ-only. `AgedBlindBoxes::for_return` counts a spouse §63(f) box on
    /// no other status, so on MFS the question was asked — and, before this, REFUSED — on a return
    /// where its answer could never move a figure. The prompt scope must track the CONSUMER's scope.
    #[test]
    fn the_spouse_death_gate_is_asked_only_on_mfj() {
        use crate::tax::questions::{SkippableId, SKIPPABLE_QUESTIONS};
        let q = SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::SpouseDiedDuringYear)
            .expect("the spouse death gate is a skippable");
        let with_spouse = |fs: FilingStatus| {
            let mut r = ri();
            r.filing_status = fs;
            r.header.spouse = Some(crate::tax::return_inputs::Person::default());
            r
        };
        assert!((q.live)(&with_spouse(FilingStatus::Mfj)), "MFJ: asked");
        assert!(
            !(q.live)(&with_spouse(FilingStatus::Mfs)),
            "MFS: the spouse box is not the taxpayer's checkbox, so the question is inert"
        );
        let mut single = ri();
        single.filing_status = FilingStatus::Single;
        assert!(!(q.live)(&single), "no spouse: nowhere to record it");
    }

    /// The skippable registry ids, as strings — a tiny helper so the test above can assert membership
    /// without importing the whole registry surface.
    fn btctax_skippables() -> impl Iterator<Item = crate::tax::questions::SkippableId> {
        crate::tax::questions::SKIPPABLE_QUESTIONS
            .iter()
            .map(|s| s.id)
    }

    #[test]
    fn mfs_without_spouse_itemize_answer_refuses() {
        let mut r = ri();
        r.filing_status = FilingStatus::Mfs; // mfs_spouse_itemizes defaults to None
        assert_eq!(reason(&r), Some(RefuseReason::MfsSpouseItemizeUnknown));
        // Answered → no refusal.
        r.mfs_spouse_itemizes = Some(false);
        assert_eq!(reason(&r), None);
    }

    #[test]
    fn salt_sales_tax_without_election_refuses() {
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            salt_sales_tax_amount: dec!(2000),
            salt_use_sales_tax: Some(false), // amount set but election OFF → input error
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::SaltSalesTaxWithoutElection));
        // Election ON → no refusal.
        r.schedule_a.as_mut().unwrap().salt_use_sales_tax = Some(true);
        assert_eq!(reason(&r), None);
    }

    #[test]
    fn hsa_and_ira_refuse() {
        let mut a = ri();
        a.sch1.hsa_activity = Some(true);
        assert_eq!(reason(&a), Some(RefuseReason::HsaActivityUnsupported));
        let mut b = ri();
        b.sch1.ira_deduction_claimed = dec!(6000);
        assert_eq!(reason(&b), Some(RefuseReason::IraDeductionClaimed));
    }

    /// ★ `FormQuestion::neutral` must be the answer that CLEARS — a property, not a comment.
    ///
    /// The field was introduced declared-but-unchecked: nothing asserted that a question's `neutral`
    /// value is actually the one requiring no adjustment. A wrong polarity is silent and severe — the
    /// registry loop and `income answer` both write `neutral` as the "nothing to see here" reply, so an
    /// inverted flag would auto-answer a filer INTO an unmodeled add-back. Held here for every entry.
    ///
    /// Mutation: flip `neutral` on any `FORM_QUESTIONS` entry and this reds.
    #[test]
    fn answering_every_live_question_neutral_leaves_no_declaration_refusal() {
        for q in FORM_QUESTIONS {
            let mut r = scenario_for(q.id);
            // ★ Same `is_none()` rule as the sibling property test above: never overwrite an answer the
            //   scenario pinned to make `q` live in the first place.
            for other in FORM_QUESTIONS {
                if (other.live)(&r) && (other.get)(&r).is_none() {
                    (other.set)(&mut r, other.neutral);
                }
            }
            // ★★ R3 — ONE CENSUS ROW IS DELIBERATELY NOT LIVE YET (Form 1098 → T9). Its amount is
            //    collected today by a scalar, so a `No` on the row would contradict a figure already
            //    entered and a `Yes` would demand a transcription section that does not exist. A
            //    never-live entry cannot be exercised by this property — but the skip is DERIVED
            //    from the census's own liveness predicate, not from a name list, so the moment T9
            //    flips `row_is_live` the entry re-enters this loop with no edit here. ★ That is
            //    exactly what happened to `Form1098e` at T5: it left this skip on its own.
            //    Anything else that is not live in its own scenario is a scenario bug and still
            //    fails.
            if !(q.live)(&r) {
                let row = crate::tax::document_census::row_of_question(q.id);
                assert!(
                    row.is_some_and(|row| !crate::tax::document_census::row_is_live(&r, row)),
                    "{:?} is not live in its own scenario and is not a census row awaiting its \
                     screen (T9)",
                    q.id
                );
                continue;
            }
            assert!((q.live)(&r), "{:?} must be live in its own scenario", q.id);
            assert_eq!(
                (q.get)(&r),
                Some(q.neutral),
                "{:?} must hold its neutral answer",
                q.id
            );
            // Every declaration answered neutrally ⇒ no declaration-attributable refusal survives.
            let got = reason(&r);
            for other in FORM_QUESTIONS {
                assert_ne!(
                    got.as_ref(),
                    Some(&other.unanswered),
                    "{:?}: answering everything neutral still refused as {:?} unanswered",
                    q.id,
                    other.id
                );
            }
            assert_ne!(
                got,
                Some(RefuseReason::AmtNonQualifiedDwelling),
                "{:?}: neutral polarity is inverted for the line-3 dwelling declaration",
                q.id
            );
            assert_ne!(
                got,
                Some(RefuseReason::AmtCarryoverDiverges),
                "{:?}: neutral polarity is inverted for the line-2k carryover declaration",
                q.id
            );
            assert_ne!(
                got,
                Some(RefuseReason::AmtDepreciationDiverges),
                "{:?}: neutral polarity is inverted for the line-2l depreciation declaration",
                q.id
            );
            assert_ne!(
                got,
                Some(RefuseReason::ForeignTrust),
                "{:?}: neutral polarity is inverted for the foreign-trust declaration",
                q.id
            );
            assert_ne!(
                got,
                Some(RefuseReason::DualStatusAlienUnsupported),
                "{:?}: neutral polarity is inverted for the dual-status declaration",
                q.id
            );
        }
    }

    /// ★★★ **K17 — the Capital Loss Carryover Worksheet's two HEADER conditions gate the return, in
    /// both directions, and ONLY when a carryforward is actually brought in.**
    ///
    /// The two sentences the worksheet prints above line 1 are governing conditions, and neither was
    /// transcribed anywhere: the conformance checker's completeness half reads only physical lines
    /// beginning `N.`, so both were invisible to it and it stayed green. They became load-bearing when
    /// `--write-carryover` learned to roll the §1212(b) figure — before that a mis-attributed or
    /// unreduced carryover was the filer's own bad input; after it btctax re-emits the figure as its
    /// OWN `Computed` value on next year's sworn Schedule D lines 6/14.
    ///
    /// **Each half asserts its own PREMISE before its outcome.** A fixture that quietly stopped
    /// reaching the gate would otherwise "pass" by refusing for some other reason, or by not being
    /// screened at all — which is how a vacuous KAT gets written.
    ///
    /// Mutations that MUST red:
    ///   (a) default either declaration to `Some(false)` in `ReturnInputs::default` ⇒ the unanswered
    ///       halves go green while the return files with a carryover btctax cannot vouch for;
    ///   (b) narrow `carryforward_in_present` with a taxable-income term ⇒ the floor half escapes;
    ///   (c) delete either value-refusal in `screen_inputs` ⇒ the `Some(true)` half reds.
    #[test]
    fn a_prior_joint_return_or_excluded_canceled_debt_refuses_when_unanswered() {
        use crate::tax::questions::{question_is_live, QuestionId};
        use crate::tax::types::Carryforward;

        // A return that carries a loss IN, with every OTHER live declaration answered neutral.
        // ★ Deliberately at POSITIVE taxable income ($60,000 of wages against a $14,600 standard
        //   deduction), so that the floor half at the end is the only fixture a taxable-income-gated
        //   liveness predicate would drop — that mutation then reds THERE and nowhere else.
        let with_carryforward = || {
            let mut r = ri();
            r.w2s = vec![W2 {
                box1_wages: dec!(60000),
                ..Default::default()
            }];
            r.capital_loss_carryforward_in = Carryforward {
                short: dec!(2000),
                long: Usd::ZERO,
            };
            r.amt_carryover_same_as_regular = Some(true);
            r
        };

        // ── PREMISE: both questions are LIVE exactly when a carryforward-in exists. ──
        let none_in = ri();
        assert_eq!(
            none_in.capital_loss_carryforward_in,
            Carryforward::default(),
            "premise: the baseline fixture brings no loss in"
        );
        for q in [
            QuestionId::CarryoverIncludesSpousesJointLoss,
            QuestionId::ExcludedCanceledDebt,
        ] {
            assert!(
                !question_is_live(q, &none_in),
                "{q:?} must NOT be asked of a return with no carryforward-in"
            );
            assert!(
                question_is_live(q, &with_carryforward()),
                "{q:?} MUST be asked of a return that brings a loss in"
            );
        }
        assert_eq!(
            reason(&none_in),
            None,
            "premise: without a carryforward-in the baseline return screens clean, so any refusal \
             below is caused by the carryforward and not by the fixture"
        );

        // ── UNANSWERED (`None`) ⇒ refuse, one reason each. ──
        let mut r = with_carryforward();
        r.excluded_canceled_debt = Some(false); // isolate the sourcing question
        assert_eq!(
            r.carryover_includes_spouses_joint_loss, None,
            "premise: the sourcing declaration starts unanswered"
        );
        assert_eq!(
            reason(&r),
            Some(RefuseReason::JointReturnCarryoverDeclarationUnanswered)
        );

        let mut r = with_carryforward();
        r.carryover_includes_spouses_joint_loss = Some(false); // isolate the canceled-debt question
        assert_eq!(
            r.excluded_canceled_debt, None,
            "premise: the canceled-debt declaration starts unanswered"
        );
        assert_eq!(
            reason(&r),
            Some(RefuseReason::ExcludedCanceledDebtDeclarationUnanswered)
        );

        // ── ANSWERED NEUTRAL (`Some(false)`) ⇒ the return screens clean. ──
        let mut r = with_carryforward();
        r.carryover_includes_spouses_joint_loss = Some(false);
        r.excluded_canceled_debt = Some(false);
        assert_eq!(
            reason(&r),
            None,
            "answering both neutrally must leave nothing behind — a gate with no exit is a brick"
        );

        // ── ANSWERED ADVERSELY (`Some(true)`) ⇒ refuse, with the ADVERSE reason, not the unanswered one. ──
        let mut r = with_carryforward();
        r.carryover_includes_spouses_joint_loss = Some(true);
        r.excluded_canceled_debt = Some(false);
        assert_eq!(
            reason(&r),
            Some(RefuseReason::JointReturnCarryoverAttributionUnknown),
            "btctax stores ONE carryover per return and cannot split a joint one by spouse"
        );

        let mut r = with_carryforward();
        r.carryover_includes_spouses_joint_loss = Some(false);
        r.excluded_canceled_debt = Some(true);
        assert_eq!(
            reason(&r),
            Some(RefuseReason::ExcludedCanceledDebtAttributeReduction),
            "§108(b)(2)(G) reduces the carryover and btctax models none of §108(b)"
        );

        // ── THE FLOOR CASE — the household lift (A) admits — is gated too. ──
        // ★ This is mutation (b)'s target: a liveness predicate narrowed with a taxable-income term
        //   would let exactly this return through, and it is the one `--write-carryover` will roll.
        let mut floor = with_carryforward();
        floor.w2s = vec![W2 {
            box1_wages: dec!(1000), // wiped out by the standard deduction ⇒ taxable income 0
            ..Default::default()
        }];
        assert!(
            question_is_live(QuestionId::CarryoverIncludesSpousesJointLoss, &floor),
            "a wiped-out year with a loss brought in is EXACTLY the household the roll persists — \
             the questions must not vanish for it"
        );
        assert_eq!(
            reason(&floor),
            Some(RefuseReason::JointReturnCarryoverDeclarationUnanswered)
        );
    }

    /// ★ ALL THREE Form 6251 VALUE-refusals must respect the liveness of their unanswered half.
    ///
    /// A `Some(false)` left over from a trigger that has since gone away — the mortgage paid off, the
    /// carryforward used up, the Schedule C wound down — describes an add-back that is structurally $0.
    /// Refusing on it is an EXIT-LESS brick: the question is no longer asked, so the filer has no way to
    /// change the answer. Each gate reads the registry via `question_is_live`, so this holds by
    /// construction rather than by three hand-copied predicates.
    ///
    /// Mutation: drop any one `question_is_live(..) &&` conjunct in `screen_inputs` and its arm reds.
    #[test]
    fn an_adverse_answer_on_a_no_longer_live_question_does_not_brick_the_return() {
        use crate::tax::return_inputs::{ScheduleAInputs, ScheduleCInputs};

        // line 3 — adverse, but no mortgage interest was deducted.
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: Usd::ZERO,
            mortgage_dwelling_is_amt_qualified: Some(false),
            salt_real_estate: dec!(4000),
            ..Default::default()
        });
        assert_ne!(
            reason(&r),
            Some(RefuseReason::AmtNonQualifiedDwelling),
            "line 3: a $0 add-back must not brick the return"
        );

        // line 2k — adverse, but no capital-loss carryforward survives.
        let mut r = ri();
        r.capital_loss_carryforward_in = crate::tax::types::Carryforward::default();
        r.amt_carryover_same_as_regular = Some(false);
        assert_ne!(
            reason(&r),
            Some(RefuseReason::AmtCarryoverDiverges),
            "line 2k: no carryforward means no divergence to add back"
        );

        // line 2l — adverse, but the Schedule C claims no expenses.
        let mut r = ri();
        r.schedule_c = Some(ScheduleCInputs {
            business_description: "Bitcoin mining".to_string(),
            naics_code: "518210".to_string(),
            expenses: Usd::ZERO,
            is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
            ..Default::default()
        });
        r.amt_depreciation_same_as_regular = Some(false);
        assert_ne!(
            reason(&r),
            Some(RefuseReason::AmtDepreciationDiverges),
            "line 2l: $0 of expenses cannot contain depreciation"
        );
    }

    /// ★ REGRESSION — the line-3 VALUE-refusal must respect the SAME liveness as its unanswered half.
    ///
    /// A Schedule A with an adverse `Some(false)` but **no mortgage interest** has a structurally $0
    /// line-3 add-back (i6251 line 3 is conditioned on having *deducted* home mortgage interest), so
    /// refusing it is a permanent, exit-less brick for no tax reason. Reachable two ways: the filer
    /// paid off or sold the boat and the stale answer persisted in the vault, or they answered the
    /// mixed-use question `Some(false)`, zeroing line 8a.
    ///
    /// Mutation: drop the `mortgage_interest_1098 > 0` conjunct from the `AmtNonQualifiedDwelling`
    /// value-refusal and this reds.
    #[test]
    fn an_adverse_dwelling_answer_without_deducted_mortgage_interest_does_not_brick_the_return() {
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: Usd::ZERO, // nothing deducted ⇒ line 3 adds back nothing
            mortgage_dwelling_is_amt_qualified: Some(false), // …yet answered adversely
            salt_real_estate: dec!(4000),
            ..Default::default()
        });
        assert_ne!(
            reason(&r),
            Some(RefuseReason::AmtNonQualifiedDwelling),
            "a $0 line-3 add-back must not produce an exit-less refusal"
        );
        // And the question is not merely silently dropped — it is NOT LIVE, so no unanswered refusal
        // fires for it either. (Both halves must agree, which is the point of sharing the predicate.)
        assert_ne!(
            reason(&r),
            Some(RefuseReason::AmtQualifiedDwellingUnanswered),
            "the question is not live, so its unanswered refusal must not fire"
        );
    }

    /// ★★★ **P7 / SCHEDULE D LINE 20 — THE TERNARY.** unanswered ⇒ refuse, "yes, filing 4952" ⇒
    /// refuse, "no" ⇒ compute.
    ///
    /// The line reads *"Are lines 18 and 19 both zero or blank **and you are not filing Form 4952**?"*
    /// btctax checked **Yes** unconditionally on every both-gains return — the lines-18/19 half it
    /// could vouch for, the Form 4952 half it could not, because nothing on the return recorded it and
    /// no question ever asked. That is testimony invented by software on a return signed under §6065.
    ///
    /// ★ It is a PROVENANCE defect, not usually a numeric one, which is why no value test could ever
    /// have found it: a filer with no margin borrowing genuinely is not filing Form 4952. When it IS
    /// wrong it is wrong in the understating direction — the Schedule D Tax Worksheet the "No" branch
    /// leads to subtracts Form 4952 line 4g at its line 4.
    ///
    /// **B1 mutations, each observed RED before the fix landed:**
    /// - delete the `FilingForm4952` entry's liveness (`live: |_| false`) ⇒ the `None` row reds;
    /// - delete the `Some(true)` value-refusal ⇒ the filing-4952 row reds, i.e. btctax computes a
    ///   return on the wrong worksheet.
    #[test]
    fn the_form_4952_declaration_refuses_unanswered_and_refuses_yes_but_computes_on_no() {
        let f = |ans: Option<bool>| {
            let mut r = ri();
            r.filing_form_4952 = ans;
            reason(&r)
        };
        assert_eq!(
            f(None),
            Some(RefuseReason::Form4952DeclarationUnanswered),
            "unanswered ⇒ refuse: Schedule D line 20 would otherwise swear to a fact nobody asked"
        );
        assert_eq!(
            f(Some(true)),
            Some(RefuseReason::Form4952Required),
            "filing Form 4952 ⇒ line 20 is NO ⇒ the Schedule D Tax Worksheet, which btctax does not \
             fill; computing on the QDCGT worksheet instead would UNDERSTATE"
        );
        assert_eq!(f(Some(false)), None, "not filing one ⇒ compute");
    }

    /// ★★★ **P7 — THE SCHEDULE A LINE 9 BOUND.** btctax now collects §163(d) investment interest and
    /// prints it on line 9, and it fills no Form 4952 — so the amount is deductible in full ONLY
    /// under i4952's own exception: *"You don't have to file Form 4952 if … your investment interest
    /// expense is not more than your investment income from interest and ordinary dividends minus any
    /// qualified dividends."*
    ///
    /// Above that ceiling Form 4952 IS required whatever the filer answered, because §163(d)(1) caps
    /// the deduction at net investment income — a figure only that form computes. Deducting the whole
    /// amount would UNDERSTATE. This is a BOUND on a collected value, not a computation: btctax still
    /// builds no Form 4952.
    ///
    /// **B1 mutations, each observed RED:**
    /// - delete the bound ⇒ the over-ceiling row reds with `None` (the excess is silently deducted);
    /// - subtract qualified dividends TWICE, or omit the subtraction ⇒ the boundary row reds.
    #[test]
    fn investment_interest_above_the_i4952_exception_ceiling_refuses() {
        use crate::tax::return_inputs::{Form1099Div, Form1099Int, ScheduleAInputs};
        // Investment income for the exception: $10,000 interest + $4,000 ordinary dividends − $1,000
        // qualified dividends = a $13,000 ceiling.
        //
        // ★ The fixture must ITEMIZE or it proves nothing: this refusal now lives behind
        //   `deduction_is_itemized` (phase-2 review R5a), and $13,000 of investment interest alone
        //   loses to the $14,600 standard deduction. $10,000 of real-estate tax carries it over.
        let build = |amount: Usd, treasury: Usd, qualified: Usd| {
            let mut r = ri();
            r.filing_form_4952 = Some(false);
            r.int_1099 = vec![Form1099Int {
                payer: "Bank".into(),
                box1_interest: dec!(10000),
                box3_treasury_interest: treasury,
                ..Default::default()
            }];
            r.div_1099 = vec![Form1099Div {
                payer: "Broker".into(),
                box1a_ordinary: dec!(4000),
                box1b_qualified: qualified,
                ..Default::default()
            }];
            r.schedule_a = Some(ScheduleAInputs {
                mortgage_interest_1098: Usd::ZERO,
                investment_interest: amount,
                salt_real_estate: dec!(10000),
                ..Default::default()
            });
            r
        };
        let at = |amount: Usd| {
            let (itemized, reason) = absolute_reason(&build(amount, Usd::ZERO, dec!(1000)));
            assert!(itemized, "fixture premise: this filer must ITEMIZE");
            reason
        };

        assert_eq!(
            at(dec!(13000)),
            None,
            "exactly at the ceiling, i4952's exception still applies — no Form 4952 required"
        );
        assert_eq!(
            at(dec!(13000.01)),
            Some(RefuseReason::Form4952Required),
            "a cent over and Form 4952 IS required: §163(d)(1) caps the deduction at net investment \
             income, which only that form computes"
        );
        // ★ And the ceiling really does SUBTRACT qualified dividends — raise them and the same
        //   $13,000 now breaks the exception. Without the subtraction this row passes.
        let (_, raised) = absolute_reason(&build(dec!(13000), Usd::ZERO, dec!(2000)));
        assert_eq!(
            raised,
            Some(RefuseReason::Form4952Required),
            "qualified dividends must be SUBTRACTED from the ceiling"
        );

        // ★★ R5b — 1099-INT BOX 3 COUNTS. Treasury obligation interest is NOT a subset of box 1
        //    (`sum_taxable_interest` says so in terms, and 1040 line 2b is box 1 + box 3), and it is
        //    unambiguously "investment income from interest" under i4952's exception. Summing box 1
        //    alone under-counted the ceiling and refused a T-bill ladder that plainly satisfies it.
        //    MUTATION: drop `+ i.box3_treasury_interest` from the ceiling and this row reds.
        let (itemized, with_treasury) =
            absolute_reason(&build(dec!(20000), dec!(10000), dec!(1000)));
        assert!(itemized, "fixture premise: this filer must ITEMIZE");
        assert_eq!(
            with_treasury, None,
            "★ $10,000 of BOX 3 Treasury interest raises the ceiling to $23,000, so $20,000 of \
             investment interest is within i4952's exception and must NOT refuse"
        );

        // ★★ R5c — THE MESSAGE, WHICH NO TEST PINNED. It shipped malformed: a string literal
        //    missing its line continuations, so ~20-space runs printed mid-sentence to the filer.
        //    Every other new refusal in this phase has a message test; this one did not, which is
        //    exactly why the defect survived to review. The whitespace assertion is the kill-test
        //    for the defect itself, not just for the content.
        let st = LedgerState::default();
        let over = build(dec!(13000.01), Usd::ZERO, dec!(1000));
        let ar = assemble_absolute(&over, &st, &params(), &tbl(), 2024);
        let detail = crate::tax::return_1040::screen_absolute(
            &over,
            &ar,
            &params(),
            &st,
            2024,
            crate::forms::InformationReturnRegime::NONE,
        )
        .expect("over the ceiling refuses")
        .detail;
        assert!(
            !detail.contains("   "),
            "no run of 3+ spaces may reach the filer — that is the missing-continuation defect: \
             {detail}"
        );
        let lower = detail.to_ascii_lowercase();
        for phrase in [
            "understate",         // the direction a full line-9 deduction fails in
            "§163(d)(1)",         // the statute that caps it
            "form 4952",          // the form that computes the cap
            "standard deduction", // the scoping this refusal now carries
        ] {
            assert!(
                lower.contains(phrase),
                "the line-9 refusal must say {phrase:?}; got: {detail}"
            );
        }

        // ★★★ THE CRITICAL'S NEGATIVE HALF (phase-2 review R5a). The crypto-margin renter — this
        //     product's core audience. Bitcoin yields no interest and no ordinary dividends, so the
        //     ceiling is $0 and ANY line-9 entry used to refuse. Here the whole Schedule A loses to
        //     the standard deduction, line 9 never prints, and the return must compute.
        let mut renter = ri();
        renter.filing_form_4952 = Some(false);
        renter.schedule_a = Some(ScheduleAInputs {
            investment_interest: dec!(5000),
            ..Default::default()
        });
        let (itemized, reason) = absolute_reason(&renter);
        assert!(
            !itemized,
            "fixture premise: $5,000 of margin interest must LOSE to the $14,600 standard deduction"
        );
        assert_eq!(
            reason, None,
            "★ a STANDARD-deduction filer must COMPUTE even though their margin interest exceeds a \
             $0 ceiling: line 9 never prints on their return, so nothing is over-deducted"
        );
    }

    /// ★★★ **P1 / §163(h)(3)(B) — THE ACQUISITION-DEBT CEILING TERNARY.** unanswered ⇒ refuse,
    /// ADVERSE ⇒ refuse with its OWN reason, neutral ⇒ compute.
    ///
    /// This is the branch that exists because btctax deducts 100% of the Form 1098 amount and the
    /// statute caps qualifying debt at $750,000 ($375,000 MFS) post-2017 / $1,000,000 ($500,000 MFS)
    /// pre-2018. **Neither oracle can catch the understatement** — both take Schedule A line 8a as an
    /// INPUT (§G-9) — so this test is the only instrument that sees it.
    ///
    /// **B1 mutations, each observed RED before the fix landed:**
    /// - delete the `MortgageWithinDebtLimit` entry from `FORM_QUESTIONS` ⇒ the `None` arm reds
    ///   (the registry loop is what raises `MortgageDebtLimitUnanswered`);
    /// - delete the `MortgageOverDebtLimit` block from `screen_inputs` ⇒ the `Some(false)` arm reds
    ///   with `None`, i.e. the over-limit filer silently deducts 100% again.
    #[test]
    fn the_acquisition_debt_limit_refuses_unanswered_and_adverse_but_computes_when_neutral() {
        use crate::tax::return_inputs::ScheduleAInputs;
        let sched = |interest: Usd, ans: Option<bool>| {
            let mut r = ri();
            r.schedule_a = Some(ScheduleAInputs {
                mortgage_interest_1098: interest,
                mortgage_all_used_to_buy_build_improve: Some(true),
                mortgage_dwelling_is_amt_qualified: Some(true),
                mortgage_within_debt_limit: ans,
                ..Default::default()
            });
            r
        };
        // The [RAN] vector's notional $2,000,000 loan — $130,000 of interest itemizes easily.
        let limit = |ans: Option<bool>| absolute_reason(&sched(dec!(130000), ans)).1;

        assert_eq!(
            reason(&sched(dec!(130000), None)),
            Some(RefuseReason::MortgageDebtLimitUnanswered),
            "unanswered ⇒ refuse: btctax collects the INTEREST, never the debt balance, so it cannot \
             tell whether §163(h)(3)(B) caps this filer"
        );
        assert_eq!(
            limit(Some(false)),
            Some(RefuseReason::MortgageOverDebtLimit),
            "ADVERSE ⇒ refuse: the Pub. 936 worksheet output is unmodelled, so the full 1098 amount \
             would UNDERSTATE and a $0 would OVERSTATE — and neither is disclosable on Schedule A"
        );
        assert_eq!(limit(Some(true)), None, "neutral ⇒ compute at the full 8a");

        // ★★★ THE CRITICAL'S NEGATIVE HALF (phase-2 review R2). The December-closing jumbo
        //     homebuyer: a $1M post-2017 loan closed late in the year yields ONE month of interest,
        //     an itemized total under the $14,600 standard deduction, and therefore NO line 8a at
        //     all. Nothing is sworn, and btctax can compute this return exactly.
        //
        //     Before the fix this filer had NO honest answer — `None` refused as unanswered,
        //     `Some(false)` refused here, and `Some(true)` would have been false testimony under
        //     §6065. That is the trap, and this row is what reds if the refusal ever migrates back
        //     to `screen_inputs`.
        let (itemized, r) = absolute_reason(&sched(dec!(5500), Some(false)));
        assert!(
            !itemized,
            "fixture premise: $5,500 of interest must LOSE to the $14,600 standard deduction — if \
             this trips, the fixture stopped testing the standard-deduction filer and the row below \
             proves nothing"
        );
        assert_eq!(
            r, None,
            "★ a STANDARD-deduction filer who truthfully answers 'over the limit' must COMPUTE: \
             line 8a never prints on their return, so there is no wrong number to swear to"
        );
    }

    /// ★★ **The over-limit refusal must carry BOTH failure directions and the cure.** A refusal that
    /// named only one direction would read as an invitation to take the other — and taking the "full
    /// 1098" direction is the understatement this whole item exists to stop. The message is the entire
    /// remedy here (nothing computes from the answer), so the message is what the test pins.
    ///
    /// B1 mutation: drop any one of the pinned phrases from the `MortgageOverDebtLimit` detail and the
    /// matching assertion reds by name.
    #[test]
    fn the_over_limit_refusal_states_both_directions_and_names_the_pub_936_worksheet() {
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(130000),
            mortgage_all_used_to_buy_build_improve: Some(true),
            mortgage_dwelling_is_amt_qualified: Some(true),
            mortgage_within_debt_limit: Some(false),
            ..Default::default()
        });
        let st = LedgerState::default();
        let ar = assemble_absolute(&r, &st, &params(), &tbl(), 2024);
        assert!(
            ar.deduction_is_itemized,
            "fixture premise: $130,000 of interest itemizes"
        );
        let refusal = crate::tax::return_1040::screen_absolute(
            &r,
            &ar,
            &params(),
            &st,
            2024,
            crate::forms::InformationReturnRegime::NONE,
        )
        .expect("over-limit refuses on an ITEMIZING return");
        assert_eq!(refusal.reason, RefuseReason::MortgageOverDebtLimit);
        let d = refusal.detail.to_ascii_lowercase();
        for phrase in [
            "understate", // deducting the full 1098 amount
            "overstate",  // zeroing line 8a
            "pub. 936",   // the cure the instructions prescribe
            "deductible home mortgage interest worksheet",
            "mortgage_interest_deductible", // the input that will close this branch
            "$750,000",
            "$1,000,000",
            "fair market value",
        ] {
            assert!(
                d.contains(phrase),
                "the over-limit refusal must say {phrase:?}; got: {}",
                refusal.detail
            );
        }
    }

    /// ★ REGRESSION twin of the line-3 one below — the debt-limit VALUE-refusal respects the SAME
    /// liveness as its unanswered half, so a stale `Some(false)` on a Schedule A that no longer
    /// reports 1098 interest is not an exit-less brick (there is no line 8a to be wrong).
    ///
    /// B1 mutation: drop the `question_is_live(MortgageWithinDebtLimit, ..)` conjunct and this reds.
    #[test]
    fn an_over_limit_answer_without_deducted_mortgage_interest_does_not_brick_the_return() {
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: Usd::ZERO, // nothing on line 8a to be capped
            mortgage_within_debt_limit: Some(false), // …yet a stale adverse answer
            salt_real_estate: dec!(4000),
            ..Default::default()
        });
        assert_ne!(
            reason(&r),
            Some(RefuseReason::MortgageOverDebtLimit),
            "no deducted mortgage interest ⇒ no line 8a to cap ⇒ no exit-less refusal"
        );
        assert_ne!(
            reason(&r),
            Some(RefuseReason::MortgageDebtLimitUnanswered),
            "…and the unanswered half must agree: the question is not live either"
        );
    }

    /// ★ THE TERNARY for Form 6251's THREE declarations: unanswered ⇒ refuse, ADVERSE ⇒ refuse,
    /// neutral ⇒ compute. The adverse branch is the one a passing test would hide — computing with a
    /// missing line-3 / line-2k / line-2l add-back UNDERSTATES the tax, which is the one direction this
    /// project never permits.
    ///
    /// Mutation: delete either value-refusal in `screen_inputs` and the matching `Some(false)`
    /// assertion below reds.
    #[test]
    fn form6251_declarations_refuse_unanswered_and_adverse_but_compute_when_neutral() {
        use crate::tax::return_inputs::ScheduleAInputs;

        // ── line 3 — the AMT qualified-dwelling declaration ──
        let dwelling = |ans: Option<bool>| {
            let mut r = ri();
            r.schedule_a = Some(ScheduleAInputs {
                mortgage_interest_1098: dec!(9000),
                mortgage_all_used_to_buy_build_improve: Some(true),
                // Reporting 1098 interest also makes the §163(h)(3)(B) debt-limit question live.
                // Answered NEUTRAL here so this test keeps testing the Form 6251 declarations.
                mortgage_within_debt_limit: Some(true),
                mortgage_dwelling_is_amt_qualified: ans,
                ..Default::default()
            });
            reason(&r)
        };
        assert_eq!(
            dwelling(None),
            Some(RefuseReason::AmtQualifiedDwellingUnanswered),
            "unanswered ⇒ refuse: btctax cannot guess which dwelling the 1098 relates to"
        );
        assert_eq!(
            dwelling(Some(false)),
            Some(RefuseReason::AmtNonQualifiedDwelling),
            "ADVERSE ⇒ refuse: the §56(b)(1)(C) add-back is unmodelled, so computing would UNDERSTATE"
        );
        assert_eq!(dwelling(Some(true)), None, "neutral ⇒ compute");

        // ── line 2k — the AMT capital-loss-carryover declaration ──
        let carryover = |ans: Option<bool>| {
            let mut r = ri();
            r.capital_loss_carryforward_in = crate::tax::types::Carryforward {
                short: dec!(1000),
                long: Usd::ZERO,
            };
            // A carryforward-in also makes the Capital Loss Carryover Worksheet's two header
            // declarations live. Answered NEUTRAL here so this test keeps testing line 2k.
            r.carryover_includes_spouses_joint_loss = Some(false);
            r.excluded_canceled_debt = Some(false);
            r.amt_carryover_same_as_regular = ans;
            reason(&r)
        };
        assert_eq!(
            carryover(None),
            Some(RefuseReason::AmtCarryoverDeclarationUnanswered),
            "unanswered ⇒ refuse"
        );
        assert_eq!(
            carryover(Some(false)),
            Some(RefuseReason::AmtCarryoverDiverges),
            "ADVERSE ⇒ refuse: a divergent AMT twin is an add-back v1 does not model"
        );
        assert_eq!(carryover(Some(true)), None, "neutral ⇒ compute");

        // ── line 2l — the AMT depreciation declaration ──
        // ★ Added after the fold's own re-review found this arm MISSING: deleting the value-refusal at
        //   `screen_inputs` left all 2,417 tests green, so the C-2 guard was unheld — a correct fix with
        //   no test behind it is the failure mode this project keeps hitting. Mutation-verify by
        //   `if false &&`-ing the `amt_depreciation_same_as_regular == Some(false)` block.
        let deprec = |ans: Option<bool>| {
            let mut r = ri();
            r.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
                business_description: "Bitcoin mining".to_string(),
                naics_code: "518210".to_string(),
                expenses: dec!(5000),
                is_sstb: Some(false), // §G-28/B1b — the SSTB declaration is live whenever there is a business
                ..Default::default()
            });
            r.amt_depreciation_same_as_regular = ans;
            reason(&r)
        };
        assert_eq!(
            deprec(None),
            Some(RefuseReason::AmtDepreciationDeclarationUnanswered),
            "unanswered ⇒ refuse"
        );
        assert_eq!(
            deprec(Some(false)),
            Some(RefuseReason::AmtDepreciationDiverges),
            "ADVERSE ⇒ refuse: a divergent AMT depreciation amount is an add-back v1 does not model"
        );
        assert_eq!(deprec(Some(true)), None, "neutral ⇒ compute");

        // No question is live when its trigger is absent — no gratuitous questions.
        let mut bare = ri();
        bare.schedule_a = None;
        bare.schedule_c = None;
        bare.capital_loss_carryforward_in = crate::tax::types::Carryforward::default();
        assert_eq!(
            reason(&bare),
            None,
            "no 1098, no carryforward and no Schedule C ⇒ none asked"
        );
    }

    #[test]
    fn foreign_trust_refuses() {
        let mut r = ri();
        r.foreign_trust = Some(true);
        assert_eq!(reason(&r), Some(RefuseReason::ForeignTrust));
        // Some(false) / None do not refuse.
        r.foreign_trust = Some(false);
        assert_eq!(reason(&r), None);
    }

    /// ★ P9 §2.5 / §3.5 (r5 I-3) — a TRUTHFUL dual-status "yes" is UNSUPPORTED: v1 cannot do a dual-status
    /// return, and §63(c)(6)(B) zeroes a nonresident alien's standard deduction. WITHOUT this guard a "yes"
    /// would COMPUTE, taking the full standard deduction the statute denies — a silent understatement, and
    /// the untested-guard pattern on the one new refusal r5 caught scheduled by no step. (`Some(false)`
    /// proceeds; `None` is the registry's unanswered refusal.)
    #[test]
    fn dual_status_alien_yes_refuses_as_unsupported() {
        let mut r = ri();
        r.dual_status_alien = Some(true);
        assert_eq!(reason(&r), Some(RefuseReason::DualStatusAlienUnsupported));
        // Some(false) proceeds — the refusal is about the UNSUPPORTED case, not the answer's existence.
        r.dual_status_alien = Some(false);
        assert_ne!(reason(&r), Some(RefuseReason::DualStatusAlienUnsupported));
    }

    /// ★ P9 §2.2 (Fable r2 I-3) — the §164(b)(5) election ON with a $0 sales-tax amount silently collapses
    /// SALT to $0 (5a = `salt_sales_tax_amount` ONLY — income-tax withholding/estimates drop out). Refuse
    /// when income-tax SALT would otherwise be deducted. The symmetric twin of `SaltSalesTaxWithoutElection`.
    #[test]
    fn sales_tax_election_without_amount_refuses() {
        use crate::tax::return_inputs::{Owner, ScheduleAInputs, W2};
        let sched = |estimated: Usd| {
            Some(ScheduleAInputs {
                salt_use_sales_tax: Some(true),
                salt_sales_tax_amount: Usd::ZERO,
                salt_state_estimated_payments: estimated,
                ..Default::default()
            })
        };

        // (a) the estimated-payments leg refuses…
        let mut r = ri();
        r.schedule_a = sched(dec!(5000));
        let refusal =
            screen_inputs(&r, &tbl(), &params()).expect("estimated-payments SALT must refuse");
        assert_eq!(refusal.reason, RefuseReason::SalesTaxElectionWithoutAmount);
        // ★ MINOR-2 — the detail NAMES BOTH exits. **T4 fold, seam review M-2: the second exit is no
        //   longer `income answer`.** This rule joined the param-free tier in T4, so it now fires at
        //   IMPORT — and on a params-less year with no committed row `income answer` refuses ("no
        //   full-return inputs and no draft"), because `income import`, the command that just
        //   refused, is the only path that could have created the row it would be answered on. Both
        //   exits are therefore edits to the file the filer is holding: enter the amount, or clear
        //   the election key. MINOR-2's requirement — *two* exits, not one — still binds.
        assert!(
            refusal.detail.contains("income import"),
            "{}",
            refusal.detail
        );
        assert!(
            refusal.detail.contains("salt_use_sales_tax"),
            "the second exit must be reachable from the TOML: {}",
            refusal.detail
        );
        assert!(
            !refusal.detail.contains("income answer"),
            "a rule in the IMPORT tier must not prescribe `income answer`: {}",
            refusal.detail
        );

        // ★ MINOR-1 — the W-2 box-17/19 withholding leg, ALONE (no estimated payments). This is the most
        // common filer shape, and it kills the mutation "drop the W-2 Σ from the income-tax-SALT set": with
        // the leg gone, `income_tax_salt` would be $0 here and the return would compute with 5a = $0.
        let mut w = ri();
        w.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            box17_state_tax_withheld: dec!(4000),
            ..Default::default()
        }];
        w.schedule_a = sched(Usd::ZERO);
        assert_eq!(
            reason(&w),
            Some(RefuseReason::SalesTaxElectionWithoutAmount),
            "W-2 state withholding ALONE must trip the collapse guard"
        );

        // With a sales-tax amount → no collapse, no refusal.
        r.schedule_a.as_mut().unwrap().salt_sales_tax_amount = dec!(3000);
        assert_ne!(
            reason(&r),
            Some(RefuseReason::SalesTaxElectionWithoutAmount)
        );

        // Election on, $0 amount, but NO income-tax SALT to lose → nothing collapses, so NOT this refusal.
        let mut r2 = ri();
        r2.schedule_a = sched(Usd::ZERO);
        assert_ne!(
            reason(&r2),
            Some(RefuseReason::SalesTaxElectionWithoutAmount)
        );
    }

    /// ★ P9 §3.2 (r1 I-6, named r3 M-2) — Schedule B 7a "Yes" with a BLANK 7b (country names) refuses. Its
    /// detail names `income import` as the exit, NOT `income answer`: `answer` captures bools and dates,
    /// never strings, so it cannot supply the country list.
    #[test]
    fn schedule_b_foreign_country_missing_refuses_and_names_import() {
        let mut r = ri();
        r.foreign_accounts = Some(true); // 7a Yes
        r.foreign_country_names = String::new(); // 7b blank
        let refusal = screen_inputs(&r, &tbl(), &params()).expect("must refuse");
        assert_eq!(refusal.reason, RefuseReason::ScheduleBForeignCountryMissing);
        assert!(
            refusal.detail.contains("income import"),
            "names the string-capable exit: {}",
            refusal.detail
        );
        assert!(
            !refusal.detail.contains("income answer"),
            "answer cannot capture strings, so it must NOT be named: {}",
            refusal.detail
        );
        // A non-empty country list → no refusal.
        r.foreign_country_names = "Canada".into();
        assert_ne!(
            reason(&r),
            Some(RefuseReason::ScheduleBForeignCountryMissing)
        );
        // Whitespace-only is still blank (the `.trim()` in the guard).
        r.foreign_country_names = "   ".into();
        assert_eq!(
            reason(&r),
            Some(RefuseReason::ScheduleBForeignCountryMissing)
        );
    }

    // ── Review C1: a non-50%-org charitable class (gift OR carryover-in) is refused — its Pub. 526
    //    special-30%-limit ordering is unmodeled in v1, and allocating it under an independent own-% room
    //    silently OVERSTATES the deduction (the two probe scenarios below). 50%-org classes stay clean. ──
    #[test]
    fn non50org_cash_gift_refuses() {
        use crate::tax::return_inputs::{CharitableGift, ScheduleAInputs};
        // Probe 1: AGI $100k, $50k Cash60 + $30k Cash30 — the flat 30% room would allow $80k vs law's $50k.
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            charitable: vec![
                CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: dec!(50000),
                },
                CharitableGift {
                    class: CharitableClass::Cash30,
                    amount: dec!(30000),
                },
            ],
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::NonPublicCharityContribution));
        // Drop the non-50%-org gift → the pure 50%-org gift is accepted.
        r.schedule_a.as_mut().unwrap().charitable.pop();
        assert_eq!(reason(&r), None);
    }

    #[test]
    fn non50org_capgain_gift_refuses() {
        use crate::tax::return_inputs::{CharitableGift, ScheduleAInputs};
        // Probe 2: AGI $100k, $30k CapGainProp30 + $20k CapGainProp20 — own-% room would allow $50k vs $30k.
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            charitable: vec![
                CharitableGift {
                    class: CharitableClass::CapGainProp30,
                    amount: dec!(30000),
                },
                CharitableGift {
                    class: CharitableClass::CapGainProp20,
                    amount: dec!(20000),
                },
            ],
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::NonPublicCharityContribution));
    }

    #[test]
    fn non50org_carryover_in_refuses() {
        // A non-50%-org class arriving as CARRYOVER-IN (no current gift) is refused too.
        let mut r = ri();
        r.charitable_carryover_in.push(CharitableCarryItem {
            class: CharitableClass::OrdinaryProp30,
            amount: dec!(5000),
            origin_year: 2022,
            provenance: crate::tax::return_inputs::CarryProvenance::default(),
        });
        assert_eq!(reason(&r), Some(RefuseReason::NonPublicCharityContribution));
        // A 50%-org carryover vintage is fine.
        r.charitable_carryover_in[0].class = CharitableClass::OrdinaryProp50;
        assert_eq!(reason(&r), None);
    }

    // ── Review I1: a claimable-as-dependent SPOUSE limits the joint standard deduction (unmodeled in v1) —
    //    refuse rather than grant the full basic std and understate tax. ──────────────────────────────────
    #[test]
    fn dependent_spouse_flag_refuses() {
        let mut r = ri();
        r.header.can_be_claimed_as_dependent_spouse = Some(true);
        assert_eq!(reason(&r), Some(RefuseReason::DependentSpouseUnsupported));
        r.header.can_be_claimed_as_dependent_spouse = Some(false);
        assert_eq!(reason(&r), None);
    }
    // ══════════════════════════════════════════════════════════════════════════════════════════════
    // T3a — Schedule 1-A lines 5 and 14b: the CONJUNCTION, and why neither limb alone is right
    // ══════════════════════════════════════════════════════════════════════════════════════════════

    fn claiming_tips() -> Schedule1aTips {
        Schedule1aTips {
            qualified_tips_reported: dec!(3000),
            occupation_on_treasury_list: true,
            treasury_occupation_code: Some("TIP-001".into()),
            excludes_unlisted_occupation_tips: true,
            meets_qualified_tip_criteria: true,
        }
    }

    /// ★★★ **The KAT that reds against BOTH wrong predicates.** Line 5 refuses only when the part is
    /// CLAIMED *and* the filer has a trade or business. Each limb alone was tried in an earlier round
    /// and each refused a population it should not have:
    ///
    /// * `schedule_c.is_some()` alone refuses **every TY2025 mining return** — btctax's core case —
    ///   on a part whose own Caution tells the filer not to complete it.
    /// * the claim gate alone refuses **100% of Part II's population**, leaving the part unreachable
    ///   and unexercisable against the per-part oracle census.
    #[test]
    fn schedule_1a_line5_refuses_only_on_the_conjunction() {
        let table = tbl();
        let params = params();

        // (1) A W-2-only tipped employee — the form's own worked example. Claims Part II, has NO
        //     trade or business, so line 5's predicate is determinately FALSE and blank is correct.
        //     This case reds if the predicate is ever widened to the claim gate alone.
        let mut employee = ri();
        employee.schedule_1a.tips = Some(claiming_tips());
        assert!(
            screen_inputs(&employee, &table, &params).is_none(),
            "a W-2-only tipped employee with no Schedule C must NOT be refused — line 5 is tips \
             received in the course of a trade or business, and they have none"
        );

        // (2) A Schedule C household that does NOT claim Part II. Reds if the predicate is ever
        //     narrowed to `schedule_c.is_some()` alone — which would refuse every mining return.
        let mut miner = ri();
        miner.schedule_c = Some(ScheduleCInputs::default());
        assert!(
            !matches!(
                screen_inputs(&miner, &table, &params).map(|r| r.reason),
                Some(RefuseReason::Schedule1aTipsFromTradeOrBusiness)
            ),
            "a Schedule C household that never claimed Part II must not be refused for line 5"
        );

        // (3) BOTH limbs true ⇒ the amount cannot be established and a printed 0 would be sworn.
        let mut both = ri();
        both.schedule_c = Some(ScheduleCInputs::default());
        both.schedule_1a.tips = Some(claiming_tips());
        assert!(
            matches!(
                screen_inputs(&both, &table, &params).map(|r| r.reason),
                Some(RefuseReason::Schedule1aTipsFromTradeOrBusiness)
            ),
            "Part II claimed WITH a trade or business must refuse: line 5 reads 1099-NEC/MISC/K and \
             btctax has no input for any of them"
        );
    }

    /// Line 14b, the same shape one part down.
    #[test]
    fn schedule_1a_line14b_refuses_only_on_the_conjunction() {
        let table = tbl();
        let params = params();

        let mut employee = ri();
        employee.schedule_1a.overtime = Some(Schedule1aOvertime {
            qualified_overtime_reported: dec!(2000),
            is_flsa_premium_half_only: true,
            entitlement_arises_under_flsa: true,
            excludes_amounts_counted_as_tips: true,
        });
        assert!(
            screen_inputs(&employee, &table, &params).is_none(),
            "an FLSA employee with no Schedule C must NOT be refused for line 14b"
        );

        let mut both = ri();
        both.schedule_c = Some(ScheduleCInputs::default());
        both.schedule_1a.overtime = Some(Schedule1aOvertime {
            qualified_overtime_reported: dec!(2000),
            is_flsa_premium_half_only: true,
            entitlement_arises_under_flsa: true,
            excludes_amounts_counted_as_tips: true,
        });
        assert!(
            matches!(
                screen_inputs(&both, &table, &params).map(|r| r.reason),
                Some(RefuseReason::Schedule1aOvertimeFromTradeOrBusiness)
            ),
            "Part III claimed WITH a trade or business must refuse for line 14b"
        );
    }

    /// The negative-money screen reaches all three Schedule 1-A leaves. An unscreened money field is
    /// a silent fail-open, and these three are new.
    #[test]
    fn schedule_1a_money_leaves_are_negative_screened() {
        for (what, mutate) in [
            (
                "Schedule 1-A qualified tips",
                (|r: &mut ReturnInputs| {
                    r.schedule_1a.tips = Some(Schedule1aTips {
                        qualified_tips_reported: dec!(-1),
                        ..Default::default()
                    });
                }) as fn(&mut ReturnInputs),
            ),
            ("Schedule 1-A qualified overtime", |r: &mut ReturnInputs| {
                r.schedule_1a.overtime = Some(Schedule1aOvertime {
                    qualified_overtime_reported: dec!(-1),
                    ..Default::default()
                });
            }),
            ("Schedule 1-A car loan interest", |r: &mut ReturnInputs| {
                r.schedule_1a.vehicles = vec![Schedule1aVehicle {
                    interest_paid: dec!(-1),
                    ..Default::default()
                }];
            }),
        ] {
            let mut r = ri();
            mutate(&mut r);
            let got = screen_inputs(&r, &tbl(), &params()).map(|x| x.reason);
            assert!(
                matches!(got, Some(RefuseReason::NegativeAmount(ref f)) if f == what),
                "{what} must be negative-screened, got {got:?}"
            );
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ **T4 / R11 — THE PARAM-FREE TIER, AND THE CENSUS THAT KEEPS IT HONEST.**
//
// `income import` runs [`screen_param_free`] before it writes; the commit gate runs
// [`screen_inputs`]. They are the SAME body under two [`ScreenTier`]s, so R11's real hazard —
// *"a rule that exists in one and not the other"* — cannot arise by copy drift. What could still
// arise is a rule landing on the WRONG SIDE of a `tier.package` gate, or a new rule nobody checked
// from the import path. So the tier assignment below is **read off this file's own source**: the
// param-free set is every `RefuseReason` the two screen bodies name, MINUS the ones inside an
// `if let Some(…) = tier.package {` block. Nothing is hand-listed, and a rule that moves tiers
// moves in this census the same day.
// ════════════════════════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod param_free_tier {
    use super::tests::{params, ri, tbl};
    use super::*;
    use crate::tax::document_census::DocumentRow;
    use crate::tax::return_inputs::{
        Box12Entry, CharitableClass, CharitableGift, Form1099B, Form1099Div, Form1099Int, Owner,
        ReturnInputs, Schedule1aOvertime, Schedule1aTips, ScheduleAInputs, ScheduleCInputs, W2,
    };
    use rust_decimal_macros::dec;
    use std::collections::BTreeSet;

    /// This file's own text. The tier census is DERIVED from it, never typed a second time — the
    /// same discipline `line-coverage` applies to a form's extract.
    const SRC: &str = include_str!("return_refuse.rs");

    /// The body of the fn whose signature line is `sig`, up to its column-0 closing brace.
    fn body_after(sig: &str) -> &'static str {
        let start = SRC
            .find(sig)
            .unwrap_or_else(|| panic!("signature not found in the source: {sig}"));
        let rest = &SRC[start..];
        let end = rest
            .find("\n}\n")
            .unwrap_or_else(|| panic!("no column-0 closing brace after: {sig}"));
        &rest[..end]
    }

    /// Every `RefuseReason::X` named in `body`, as bare variant names.
    fn reasons_in(body: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = body;
        while let Some(i) = rest.find("RefuseReason::") {
            rest = &rest[i + "RefuseReason::".len()..];
            let n = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            if n > 0 {
                out.insert(rest[..n].to_string());
            }
        }
        out
    }

    /// Every `RefuseReason::X` inside a `guard { … }` block of `body`, brace-matched. `guard` must
    /// end with its opening brace.
    fn reasons_under_guard(body: &str, guard: &str) -> BTreeSet<String> {
        assert!(
            guard.ends_with('{'),
            "the guard text must end with its brace"
        );
        let mut out = BTreeSet::new();
        let mut from = 0usize;
        let mut blocks = 0usize;
        while let Some(i) = body[from..].find(guard) {
            let open = from + i + guard.len() - 1; // index of the `{`
            let bytes = body.as_bytes();
            let (mut depth, mut j) = (0i32, open);
            let close = loop {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break j;
                        }
                    }
                    _ => {}
                }
                j += 1;
                assert!(j < bytes.len(), "unbalanced braces after: {guard}");
            };
            out.extend(reasons_in(&body[open..close]));
            blocks += 1;
            from = close;
        }
        assert!(blocks > 0, "no block found for the guard: {guard}");
        out
    }

    /// A `RefuseReason`'s variant name, so a payload-carrying variant compares like a unit one.
    fn name_of(r: &RefuseReason) -> String {
        let s = format!("{r:?}");
        s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// Answer whatever the fixture's shape has just made live, WITHOUT touching the census (which
    /// `testonly::answer_all_live_declarations` reconciles — and two fixtures below depend on an
    /// incoherent census being left exactly as written). Iterated, because answering one question
    /// can make another live.
    fn answer_remaining(ri: &mut ReturnInputs) {
        for _ in 0..8 {
            let mut changed = false;
            for q in crate::tax::questions::FORM_QUESTIONS {
                if (q.live)(ri) && (q.get)(ri).is_none() {
                    (q.set)(ri, q.neutral);
                    changed = true;
                }
            }
            if !changed {
                return;
            }
        }
        panic!("the declarations did not settle — a liveness cycle");
    }

    fn w2(mut f: impl FnMut(&mut W2)) -> W2 {
        let mut w = W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(50000),
            ..Default::default()
        };
        f(&mut w);
        w
    }

    fn sched_c() -> ScheduleCInputs {
        ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "consulting".into(),
            ..Default::default()
        }
    }

    /// **The fixture table: one return per param-free rule.** Each is built from [`ri`] — every
    /// always-live declaration already answered — perturbed in exactly one way, then topped up by
    /// [`answer_remaining`] so the only thing left standing is the rule under test.
    fn param_free_fixtures() -> Vec<(&'static str, ReturnInputs)> {
        let mut out: Vec<(&'static str, ReturnInputs)> = Vec::new();
        let mut add = |name: &'static str, build: &dyn Fn(&mut ReturnInputs)| {
            let mut r = ri();
            build(&mut r);
            answer_remaining(&mut r);
            out.push((name, r));
        };

        add("NegativeAmount", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| w.box1_wages = dec!(-1)));
        });
        add("Schedule1aTipsFromTradeOrBusiness", &|r| {
            r.schedule_c = Some(sched_c());
            r.schedule_1a.tips = Some(Schedule1aTips {
                qualified_tips_reported: dec!(1000),
                ..Default::default()
            });
        });
        add("Schedule1aOvertimeFromTradeOrBusiness", &|r| {
            r.schedule_c = Some(sched_c());
            r.schedule_1a.overtime = Some(Schedule1aOvertime {
                qualified_overtime_reported: dec!(1000),
                ..Default::default()
            });
        });
        add("Form1099BNeedsForm8949", &|r| {
            r.documents.set(DocumentRow::B1099, Some(true));
            r.b_1099.push(Form1099B {
                payer: "Broker".into(),
                short_term_proceeds: dec!(500),
                basis_reported_and_no_adjustments: None,
                ..Default::default()
            });
        });
        // ★ R10.4 / T4b — the carried filing status, answered NO. Param-free: the refusal reads
        //   two `Option`s and no table.
        add("FilingStatusChanged", &|r| {
            r.opened_from = Some(2024);
            r.filing_status_confirmed = Some(false);
        });
        add("DocumentTypeUnsupported", &|r| {
            r.documents.set(DocumentRow::K1, Some(true));
        });
        add("DocumentDeclaredNotTranscribed", &|r| {
            r.documents.set(DocumentRow::W2, Some(true)); // declared, nothing transcribed
        });
        add("DocumentCensusContradicted", &|r| {
            r.documents.set(DocumentRow::W2, Some(false)); // "no", beside a row
            r.w2s.push(w2(|_| {}));
        });
        add("JointReturnCarryoverAttributionUnknown", &|r| {
            r.capital_loss_carryforward_in.long = dec!(3000);
            r.carryover_includes_spouses_joint_loss = Some(true);
        });
        add("ExcludedCanceledDebtAttributeReduction", &|r| {
            r.capital_loss_carryforward_in.long = dec!(3000);
            r.carryover_includes_spouses_joint_loss = Some(false);
            r.excluded_canceled_debt = Some(true);
        });
        add("ForeignTrust", &|r| r.foreign_trust = Some(true));
        add("Form4952Required", &|r| r.filing_form_4952 = Some(true));
        add("AmtNonQualifiedDwelling", &|r| {
            r.schedule_a = Some(ScheduleAInputs {
                mortgage_interest_1098: dec!(9000),
                mortgage_dwelling_is_amt_qualified: Some(false),
                ..Default::default()
            });
        });
        add("AmtCarryoverDiverges", &|r| {
            r.capital_loss_carryforward_in.long = dec!(3000);
            r.amt_carryover_same_as_regular = Some(false);
        });
        add("AmtDepreciationDiverges", &|r| {
            r.schedule_c = Some(ScheduleCInputs {
                expenses: dec!(2000),
                other_gross_receipts: dec!(9000),
                ..sched_c()
            });
            r.amt_depreciation_same_as_regular = Some(false);
        });
        add("OtherIncomeOutOfScope", &|r| {
            r.other_out_of_scope_income = Some(true)
        });
        add("DualStatusAlienUnsupported", &|r| {
            r.dual_status_alien = Some(true)
        });
        add("ScheduleBForeignCountryMissing", &|r| {
            r.foreign_accounts = Some(true);
            r.foreign_country_names = String::new();
        });
        add("SaltSalesTaxWithoutElection", &|r| {
            r.schedule_a = Some(ScheduleAInputs {
                salt_sales_tax_amount: dec!(1200),
                salt_use_sales_tax: None,
                ..Default::default()
            });
        });
        add("SalesTaxElectionWithoutAmount", &|r| {
            r.schedule_a = Some(ScheduleAInputs {
                salt_use_sales_tax: Some(true),
                salt_sales_tax_amount: Usd::ZERO,
                salt_state_estimated_payments: dec!(800),
                ..Default::default()
            });
        });
        add("NonPublicCharityContribution", &|r| {
            r.schedule_a = Some(ScheduleAInputs {
                charitable: vec![CharitableGift {
                    class: CharitableClass::Cash30,
                    amount: dec!(500),
                }],
                ..Default::default()
            });
        });
        add("DependentSpouseUnsupported", &|r| {
            r.header.can_be_claimed_as_dependent_spouse = Some(true)
        });
        add("SpouseOwnerWithoutJointReturn", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| w.owner = Owner::Spouse));
        });
        add("ScheduleCNoBusinessDescription", &|r| {
            r.schedule_c = Some(ScheduleCInputs {
                business_description: String::new(),
                ..sched_c()
            });
        });
        add("AllocatedTips", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| w.box8_allocated_tips = dec!(400)));
        });
        add("DependentCareBenefit", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| w.box10_dependent_care = dec!(400)));
        });
        add("UnsupportedBox12Code", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| {
                w.box12 = vec![Box12Entry {
                    code: "Q".into(),
                    amount: dec!(100),
                }]
            }));
        });
        add("PrivateActivityBondAmt", &|r| {
            r.documents.set(DocumentRow::Int1099, Some(true));
            r.int_1099.push(Form1099Int {
                payer: "Bank".into(),
                box9_private_activity_bond_amt: dec!(100),
                ..Default::default()
            });
        });
        add("InconsistentDividendSubset", &|r| {
            r.documents.set(DocumentRow::Div1099, Some(true));
            r.div_1099.push(Form1099Div {
                payer: "Fund".into(),
                box1a_ordinary: dec!(100),
                box1b_qualified: dec!(200),
                ..Default::default()
            });
        });
        add("UnrecapturedOrSpecialRateGain", &|r| {
            r.documents.set(DocumentRow::Div1099, Some(true));
            r.div_1099.push(Form1099Div {
                payer: "Fund".into(),
                box1a_ordinary: dec!(100),
                box2b_unrecap_1250: dec!(50),
                ..Default::default()
            });
        });
        add("HsaActivityUnsupported", &|r| {
            r.sch1.hsa_activity = Some(true)
        });
        add("IraDeductionClaimed", &|r| {
            r.sch1.ira_deduction_claimed = dec!(3000)
        });
        // ── ★★★ R3 / R4 / T5 — the document-less income door and the four box decisions. ────────
        //
        // ★ `answer_remaining` runs AFTER each builder, so a question left `None` here is answered
        //   at its neutral and the only thing standing is the rule under test. The three door
        //   questions are answered `Some(true)` explicitly where the ADVERSE answer is the rule.
        add("WagesWithoutW2", &|r| {
            // `ri()` already answers every census row `Some(false)`, which is what makes the
            // question live; `ri()` also pre-answers it `false`, so the fixture flips it.
            r.w2_wages_without_w2 = Some(true);
        });
        add("FilerRecordsDeclaredNotTranscribed", &|r| {
            r.interest_or_dividends_without_1099 = Some(true);
            r.schedule_b_filer_records.clear();
        });
        add("StateAndLocalRefundWorksheetNotComputed", &|r| {
            r.state_refund_without_1099g = Some(true);
            r.itemized_prior_year = Some(true);
        });
        add("AmortizableBondPremiumNotComputed", &|r| {
            r.documents.set(DocumentRow::Int1099, Some(true));
            r.int_1099.push(Form1099Int {
                payer: "Bank".into(),
                box1_interest: dec!(500),
                box11_bond_premium: dec!(40),
                ..Default::default()
            });
        });
        add("StatutoryEmployeeW2", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| w.box13_statutory_employee = true));
        });
        add("FamilyLeaveBenefits", &|r| {
            r.documents.set(DocumentRow::G1099, Some(true));
            r.g_1099.push(crate::tax::return_inputs::Form1099G {
                payer: "State of Example".into(),
                box10_family_leave_benefits: dec!(1200),
                ..Default::default()
            });
        });
        out
    }

    /// One return per rule that WAITS for the year's package.
    fn package_fixtures() -> Vec<(&'static str, ReturnInputs)> {
        let mut out: Vec<(&'static str, ReturnInputs)> = Vec::new();
        let mut add = |name: &'static str, build: &dyn Fn(&mut ReturnInputs)| {
            let mut r = ri();
            build(&mut r);
            answer_remaining(&mut r);
            out.push((name, r));
        };
        add("ExcessSsEmployerUnknown", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            // Over the §3101(a) cap, with no EIN to say whether it was one employer or two.
            r.w2s.push(w2(|w| {
                w.box3_ss_wages = dec!(200000);
                w.box4_ss_withheld = dec!(40000);
                w.ein = None;
            }));
        });
        add("ExcessElectiveDeferral", &|r| {
            r.documents.set(DocumentRow::W2, Some(true));
            r.w2s.push(w2(|w| {
                w.box12 = vec![Box12Entry {
                    code: "D".into(),
                    amount: dec!(99000),
                }]
            }));
        });
        add("ForeignTaxOverCeiling", &|r| {
            r.documents.set(DocumentRow::Int1099, Some(true));
            r.int_1099.push(Form1099Int {
                payer: "Bank".into(),
                box1_interest: dec!(5000),
                box6_foreign_tax: dec!(5000),
                ..Default::default()
            });
        });
        out
    }

    /// ★★★ **The census: the param-free set is READ OFF THE SOURCE, and every member of it has a
    ///     fixture that fires it through BOTH entry points.**
    ///
    /// Removing a rule from the body removes it from the census and leaves its fixture refusing
    /// nothing — red on both halves. Adding one, or moving one inside/outside a `tier.package`
    /// gate, changes the census and demands (or releases) a fixture — red until someone looks.
    #[test]
    fn every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths() {
        let body = body_after(
            "pub fn screen_inputs_tiered(ri: &ReturnInputs, tier: ScreenTier<'_>) -> Option<Refusal> {",
        );
        let census_fn =
            body_after("pub fn screen_document_census(ri: &ReturnInputs) -> Option<Refusal> {");

        let named: BTreeSet<String> = reasons_in(body)
            .union(&reasons_in(census_fn))
            .cloned()
            .collect();
        let package_gated: BTreeSet<String> =
            reasons_under_guard(body, "if let Some((tbl, _)) = tier.package {")
                .union(&reasons_under_guard(
                    body,
                    "if let Some((_, p)) = tier.package {",
                ))
                .cloned()
                .collect();
        let param_free: BTreeSet<String> = named.difference(&package_gated).cloned().collect();

        // A broken parse must be LOUD, not silently permissive.
        assert!(
            named.len() > 30 && !package_gated.is_empty(),
            "the source census parsed nothing usable: {} named, {} package-gated",
            named.len(),
            package_gated.len()
        );

        let fixtures = param_free_fixtures();
        let covered: BTreeSet<String> = fixtures.iter().map(|(n, _)| (*n).to_string()).collect();
        assert_eq!(
            covered, param_free,
            "the fixture table and the source census must name the same param-free rules \
             (left: fixtures, right: source)"
        );
        assert_eq!(
            covered.len(),
            fixtures.len(),
            "a rule is listed twice in the fixture table"
        );

        for (name, r) in &fixtures {
            let with_package = screen_inputs(r, &tbl(), &params()).map(|x| name_of(&x.reason));
            assert_eq!(
                with_package.as_deref(),
                Some(*name),
                "the commit gate must reach {name} on its own fixture"
            );
            let without = screen_param_free(r).map(|x| name_of(&x.reason));
            assert_eq!(
                without.as_deref(),
                Some(*name),
                "`income import` screens on a year with NO package — {name} must fire there too"
            );
        }
    }

    /// ★★★ **T4 fold, seam review M-2 — NO REFUSAL IN THE IMPORT TIER MAY PRESCRIBE
    ///     `income answer`.**
    ///
    /// T4 moved these rules from commit-time to import-time. On a params-less year with no committed
    /// row that exit does not exist: `income answer` refuses with *"no full-return inputs and no
    /// draft"*, and `income import` — the command that just refused — is the only path that could
    /// have created the row it would be answered on. It is the same reasoning as the unanswered
    /// tier's, one rule at a time.
    ///
    /// ★ Behavioural, not a source grep: it reads the DETAIL each fixture actually produces through
    ///   [`screen_param_free`], so a rule joining the tier with that clause reds this the same day —
    ///   and a rule whose clause is assembled at runtime (`entry_route`, a formatted exit sentence)
    ///   is covered, which a grep over literals would miss.
    ///
    /// ★ The UNANSWERED tier is exempt BY CONSTRUCTION and correctly so: it never runs at import
    ///   (`ScreenTier::unanswered_refuses`), so its registry details name `income answer` truthfully.
    #[test]
    fn no_refusal_in_the_import_tier_prescribes_income_answer() {
        let mut named: Vec<&str> = Vec::new();
        for (name, r) in param_free_fixtures() {
            let detail = screen_param_free(&r)
                .unwrap_or_else(|| panic!("{name} must refuse on its own fixture"))
                .detail;
            if detail.contains("income answer") {
                named.push(name);
            }
        }
        assert!(
            named.is_empty(),
            "these import-tier refusals prescribe `income answer`, which a refused import cannot \
             reach: {named:?}"
        );
    }

    /// The other side of the tier: a rule that reads the year's package fires at commit and is
    /// SILENT at import — R11's *"the param-dependent rules still wait for commit"*.
    #[test]
    fn a_package_dependent_rule_fires_at_commit_and_is_silent_without_the_package() {
        let body = body_after(
            "pub fn screen_inputs_tiered(ri: &ReturnInputs, tier: ScreenTier<'_>) -> Option<Refusal> {",
        );
        let package_gated: BTreeSet<String> =
            reasons_under_guard(body, "if let Some((tbl, _)) = tier.package {")
                .union(&reasons_under_guard(
                    body,
                    "if let Some((_, p)) = tier.package {",
                ))
                .cloned()
                .collect();
        let fixtures = package_fixtures();
        let covered: BTreeSet<String> = fixtures.iter().map(|(n, _)| (*n).to_string()).collect();
        assert_eq!(
            covered, package_gated,
            "every package-gated rule needs a fixture (left: fixtures, right: source)"
        );
        for (name, r) in &fixtures {
            assert_eq!(
                screen_inputs(r, &tbl(), &params())
                    .map(|x| name_of(&x.reason))
                    .as_deref(),
                Some(*name),
                "the commit gate must reach {name}"
            );
            assert_eq!(
                screen_param_free(r).map(|x| x.reason),
                None,
                "{name} reads a figure the year has no package for — it must not fire at import"
            );
        }
    }

    /// ★★★ **The UNANSWERED tier is gated, and every one of its refusals is inside the gate.**
    ///
    /// It raises `q.unanswered` from the registry rather than a literal `RefuseReason::X`, so the
    /// census above cannot see it. This counts the raising sites instead: all of them must sit
    /// inside `if tier.unanswered_refuses { … }`, or an unanswered declaration would refuse an
    /// `income import` that is the only way to create the row it would be answered on.
    #[test]
    fn every_unanswered_refusal_sits_inside_the_unanswered_gate() {
        let body = body_after(
            "pub fn screen_inputs_tiered(ri: &ReturnInputs, tier: ScreenTier<'_>) -> Option<Refusal> {",
        );
        let total = body.matches("q.unanswered.clone()").count();
        assert_eq!(
            total, 2,
            "the registry loop raises exactly two unanswered refusals"
        );
        let open = body
            .find("if tier.unanswered_refuses {")
            .expect("the unanswered tier is gated");
        let bytes = body.as_bytes();
        let brace = open + "if tier.unanswered_refuses ".len();
        let (mut depth, mut j) = (0i32, brace);
        let close = loop {
            match bytes[j] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break j;
                    }
                }
                _ => {}
            }
            j += 1;
        };
        assert_eq!(
            body[open..close].matches("q.unanswered.clone()").count(),
            total,
            "an unanswered refusal outside the gate would refuse `income import`"
        );
    }

    /// The behavioural half of the gate: an unanswered live declaration refuses at commit and does
    /// NOT refuse at import.
    #[test]
    fn an_unanswered_declaration_refuses_at_commit_and_not_at_import() {
        let mut r = ri();
        r.foreign_trust = None; // always live, and now unanswered
        assert!(
            matches!(
                screen_inputs(&r, &tbl(), &params()).map(|x| x.reason),
                Some(RefuseReason::ScheduleBPart3Unanswered)
            ),
            "the commit gate refuses an unanswered declaration: {:?}",
            screen_inputs(&r, &tbl(), &params()).map(|x| x.reason)
        );
        assert_eq!(
            screen_param_free(&r).map(|x| x.reason),
            None,
            "`income import` is the only path that CREATES the row `income answer` fills — it must \
             not demand the answers"
        );
    }
}
