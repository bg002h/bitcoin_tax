//! Full-return v1 **fail-closed refuse-guard** (Phase 1 task 4 / SPEC §4.10 / §3.4).
//!
//! A wrong full return is worse than a refusal. Any captured-but-unmodeled input that could make the
//! return *wrong* (understate tax, misstate a figure, or require a mandatory attachment v1 can't produce)
//! yields a [`Refusal`] — never a silent value. This module screens the **input-screenable** rows (those
//! decidable from `ReturnInputs` + the year tables). The **compute-dependent** rows — Schedule C net < 0,
//! Form 8615 kiddie tax (unearned income > threshold), taxable income ≤ 0 with a carryforward — and the
//! **ledger-dependent** rows — ≥2 SE earners, business-flagged crypto interest, §1250/§1202/28% crypto —
//! are screened in Phase 2/3 where the assembled income / ledger is available.
//!
//! Uses a NEW domain type (not the ledger's shared `state::BlockerKind`, which is exhaustively matched
//! across the reconcile system) — additive, per SPEC §2. A `Refusal` maps to
//! `TaxOutcome::NotComputable(..)` at the report boundary (Phase 4).
use crate::conventions::Usd;
use crate::tax::return_inputs::{
    Box12Entry, CharitableCarryItem, CharitableClass, CharitableGift, Form1099Div, Form1099G,
    Form1099Int, Owner, Payments, QbiInputs, ReturnInputs, Schedule1Inputs, ScheduleAInputs,
    ScheduleCInputs, W2,
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
    /// A claimable-as-dependent filer with unearned income over the §1(g) kiddie-tax threshold → Form 8615
    /// (the child's-rate `qdcgt_line16` would understate; the parent's rate is required — C1/F2).
    KiddieTax,
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
    /// Taxable income ≤ 0 **with a capital-loss carryforward-in** — the §1211/§1212 Capital Loss Carryover
    /// Worksheet (G22 edge) decides how much loss survives when it can't reduce an already-zero tax; v1
    /// doesn't model it, so refuse rather than write a wrong next-year carryover. A refund-only TI≤0 filer
    /// with NO carryforward is NOT refused (tax = 0, withholding refunded). Compute-dependent (needs L15).
    TaxableIncomeNonPositiveWithCarryforward,
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
        schedule_c,
        schedule_a,
        itemize_election: _,
        mfs_spouse_itemizes: _,
        sch1,
        payments,
        capital_loss_carryforward_in,
        capital_loss_carryforward_in_provenance: _, // CarryProvenance, not an amount
        charitable_carryover_in_provenance: _,
        amt_carryover_same_as_regular: _, // a declaration, not money
        amt_depreciation_same_as_regular: _, // a declaration, not money
        charitable_carryover_in,
        qbi,
        foreign_accounts: _,
        foreign_trust: _,
        fbar_filing_required: _,
        foreign_country_names: _,
        donations_had_restrictions: _, // Option<bool>, not an amount
        dual_status_alien: _,
        // MAGI add-backs — refused at the worksheet's point of need, not here (D-11).
        has_income_exclusion: _, // refused at the worksheet's point of need (D-11), not here
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
    } = ri;

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
        } = i;
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
            box4_fed_withheld,
        } = g;
        if neg(*box1_unemployment) {
            return Some("1099-G box 1 unemployment compensation");
        }
        if neg(*box4_fed_withheld) {
            return Some("1099-G box 4 federal withholding");
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
    let Schedule1Inputs {
        state_refund_taxable,
        student_loan_interest_paid,
        ira_deduction_claimed,
        hsa_activity: _,
    } = sch1;
    if neg(*state_refund_taxable) {
        return Some("Schedule 1 taxable state refund");
    }
    if neg(*student_loan_interest_paid) {
        return Some("Schedule 1 student-loan interest");
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

/// Screen the **input-screenable** refuse-guard rows (SPEC §4.10). Returns the FIRST [`Refusal`] found,
/// or `None` if nothing input-screenable trips (the compute/ledger-dependent rows are checked later).
pub fn screen_inputs(ri: &ReturnInputs, tbl: &TaxTable, p: &FullReturnParams) -> Option<Refusal> {
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
    for q in crate::tax::questions::FORM_QUESTIONS {
        if (q.live)(ri) && (q.get)(ri).is_none() {
            return refuse(q.unanswered.clone(), q.unanswered_detail);
        }
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
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::MortgageWithinDebtLimit,
        ri,
    ) && ri
        .schedule_a
        .as_ref()
        .is_some_and(|a| a.mortgage_within_debt_limit == Some(false))
    {
        return refuse(
            RefuseReason::MortgageOverDebtLimit,
            "you declared that one of the §163(h)(3)(B) home-mortgage limits applies to you — the \
             $750,000 ($375,000 married filing separately) ceiling on qualifying debt taken out after \
             December 15, 2017, the $1,000,000 ($500,000 MFS) ceiling on debt taken out on or before \
             that date, or the limit where the mortgages exceed the home's fair market value. \
             i1040sca's own instruction for line 8a is \"Only enter on line 8a the deductible mortgage \
             interest and points that were reported to you on Form 1098\", and btctax cannot figure \
             that amount. NEITHER number it could print is your return: deducting the full Form 1098 \
             figure would UNDERSTATE your tax, and entering $0 would OVERSTATE it by the whole \
             deductible portion — and Schedule A has no box that would disclose such a zero (the \
             line-8 checkbox is the mixed-use disclosure, not this one). The cure is the one the \
             instructions prescribe: work Pub. 936's Deductible Home Mortgage Interest Worksheet, \
             which produces the deductible amount for line 8a. btctax does not yet have a place to \
             enter that result — the `mortgage_interest_deductible` input is filed as FOLLOWUPS P9(a)/S2 \
             — so until it lands, file this year's Schedule A by hand. `btctax report` still runs",
        );
    }
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
                "the §164(b)(5) sales-tax election is ON but `salt_sales_tax_amount` is $0, so Schedule A \
                 line 5a would be $0 and your state/local income taxes (W-2 box 17/19 withholding, \
                 estimates, prior-year balance) drop out — enter the amount and re-run `btctax income \
                 import`, or run `btctax income answer` to turn the election off and deduct income taxes",
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
    let excess_ss_max = tbl.ss_wage_base * EMPLOYEE_OASDI_RATE; // §3101(a)/§6413(c)
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
    {
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
    if deferral_tp > p.elective_deferral_limit || deferral_sp > p.elective_deferral_limit {
        return refuse(
            RefuseReason::ExcessElectiveDeferral,
            "one person's elective deferrals exceed the §402(g) limit — the taxable excess (1040 line 1h) is unmodeled in v1",
        );
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
    if foreign_tax > ftc_ceiling_for(p, ri.filing_status) {
        return refuse(
            RefuseReason::ForeignTaxOverCeiling,
            "foreign tax exceeds the §904(j) $300/$600 no-Form-1116 ceiling — Form 1116 is out of scope",
        );
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
    use crate::tax::return_inputs::{Box12Entry, Form1099Div, Form1099Int, W2};
    use crate::tax::tables::SaltLimitation;

    // A synthetic TY2024 FullReturnParams + a table with the real SS wage base for the excess-SS MAX.
    fn params() -> FullReturnParams {
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
    fn tbl() -> TaxTable {
        crate::tax::tables::synthetic_table(2024) // ss_wage_base = 176,100 (synthetic); MAX = 10,918.20
    }
    fn ri() -> ReturnInputs {
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
        // §G-28/B1b — answered whenever the fixture has a trade or business, since the SSTB question
        // is live exactly then. Set through the same path a filer would use.
        if let Some(c) = ri.schedule_c.as_mut() {
            c.is_sstb = Some(false);
        }
        // §G-9: "did not die during the tax year". No longer REQUIRED (the death gates are class-(B)
        // skippables now — see `the_death_gates_do_not_block_a_return`), but kept so that every fixture
        // below claims the age-65 box the way a real filer would, and no existing figure moves.
        ri.header.taxpayer_died_during_year = Some(false);
        ri
    }
    fn reason(ri: &ReturnInputs) -> Option<RefuseReason> {
        screen_inputs(ri, &tbl(), &params()).map(|r| r.reason)
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
            QuestionId::MortgageAllUsedToBuyBuildImprove
            | QuestionId::AmtQualifiedDwelling
            | QuestionId::MortgageWithinDebtLimit => {
                r.schedule_a = Some(ScheduleAInputs {
                    mortgage_interest_1098: dec!(9000),
                    ..Default::default()
                });
            }
            QuestionId::AmtCarryoverSameAsRegular => {
                r.capital_loss_carryforward_in = crate::tax::types::Carryforward {
                    short: dec!(1000),
                    long: Usd::ZERO,
                };
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
            r.header.taxpayer_died_during_year = Some(false); // §G-9
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
        let limit = |ans: Option<bool>| {
            let mut r = ri();
            r.schedule_a = Some(ScheduleAInputs {
                mortgage_interest_1098: dec!(130000), // the [RAN] vector's notional $2,000,000 loan
                mortgage_all_used_to_buy_build_improve: Some(true),
                mortgage_dwelling_is_amt_qualified: Some(true),
                mortgage_within_debt_limit: ans,
                ..Default::default()
            });
            reason(&r)
        };
        assert_eq!(
            limit(None),
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
        let refusal = screen_inputs(&r, &tbl(), &params()).expect("over-limit refuses");
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
        // ★ MINOR-2 — the detail NAMES both exits: `income import` (to enter the amount — `answer` can't
        // capture a dollar figure) and `income answer` (to flip the skippable election off).
        assert!(
            refusal.detail.contains("income import"),
            "{}",
            refusal.detail
        );
        assert!(
            refusal.detail.contains("income answer"),
            "{}",
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
}
