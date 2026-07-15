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
use crate::tax::packet::{Ssn, SsnError};
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
    /// A captured SSN is not nine digits. Every IRS form prints the SSN in a fixed-width comb cell, so a
    /// value that cannot be canonicalized cannot be printed — and §3.4 makes an unprintable identity an
    /// uncomputable line, not a best-effort cell. Carries WHO ("taxpayer" / "spouse" / a dependent's
    /// name), **never the digits**: an SSN in an error string is a PII leak.
    ///
    /// An *uncaptured* (empty) SSN is deliberately NOT this: the tax math never reads an SSN, so a
    /// household that has entered no PII still gets a report. The filable packet is what refuses it
    /// (`ReturnHeader::build` → `SsnError::Missing`).
    SsnMalformed(String),
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
    SingleEmployerExcessSs,
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
    /// The 2024 "Worksheet To See if You Should Fill in Form 6251" concludes the taxpayer must file Form
    /// 6251 — v1 does not compute the AMT, so it refuses rather than under-state (SPEC §4.11). Compute-
    /// dependent (needs AGI, QBI, and L16). A cleared worksheet leaves Schedule 2 line 2 = 0 (no refuse).
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

/// The first household SSN that was CAPTURED but cannot be canonicalized, as `(who, why)`. An empty SSN
/// is skipped — "not entered" is a packet-time refusal, not a compute-time one.
fn first_malformed_ssn(ri: &ReturnInputs) -> Option<(String, SsnError)> {
    let dependents = ri
        .header
        .dependents
        .iter()
        .map(|d| (format!("dependent {}'s", d.name), d.ssn.as_str()));
    let people = [
        ("taxpayer".to_string(), ri.header.taxpayer.ssn.as_str()),
        (
            "spouse".to_string(),
            ri.header.spouse.as_ref().map_or("", |s| s.ssn.as_str()),
        ),
    ]
    .into_iter()
    .chain(dependents);

    people.into_iter().find_map(|(who, raw)| {
        if raw.trim().is_empty() {
            return None; // uncaptured — the packet refuses this, not the report
        }
        Ssn::canonical(raw).err().map(|e| (who, e))
    })
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
        filing_status: _,
        header: _, // PII only — no money
        w2s,
        int_1099,
        div_1099,
        g_1099,
        schedule_c,
        schedule_a,
        itemize_election: _,
        mfs_spouse_itemizes: _,
        sch1,
        payments,
        capital_loss_carryforward_in,
        charitable_carryover_in,
        qbi,
        foreign_accounts: _,
        foreign_trust: _,
        foreign_country_names: _,
        dual_status_alien: _,
    } = ri;

    for w in w2s {
        let W2 {
            owner: _,
            employer: _,
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
            box13_retirement_plan: _,
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
    if let Some(c) = schedule_c {
        let ScheduleCInputs {
            owner: _,
            business_description: _,
            naics_code: _,
            accounting_method: _,
            expenses,
        } = c;
        if neg(*expenses) {
            return Some("Schedule C expenses");
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
    } = qbi;
    if neg(*reit_ptp_carryforward_in) {
        return Some("QBI REIT/PTP carryforward");
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

    // Identity integrity: a CAPTURED SSN that is not nine digits can never reach a form cell, so it
    // fails here rather than at the export boundary (`p1-ssn-normalization`). An EMPTY SSN is "not
    // entered yet" and does not block the report — only the packet.
    if let Some((who, err)) = first_malformed_ssn(ri) {
        return refuse(
            RefuseReason::SsnMalformed(who.clone()),
            format!("the {who} SSN {err} — fix it before the return can be computed"),
        );
    }

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

    // ★ P9 §2.5 (r5 I-3) — a truthful dual-status "yes" is UNSUPPORTED. VALUE-refusal (`Some(true)`);
    // WITHOUT it a "yes" computes, taking the standard deduction §63(c)(6)(B) denies a nonresident alien.
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
        if a.salt_use_sales_tax == Some(true) && a.salt_sales_tax_amount == Usd::ZERO {
            let income_tax_salt: Usd = ri
                .w2s
                .iter()
                .map(|w| w.box17_state_tax_withheld + w.box19_local_tax)
                .sum::<Usd>()
                + a.salt_state_estimated_payments
                + a.salt_prior_year_balance_paid;
            if income_tax_salt > Usd::ZERO {
                return refuse(
                    RefuseReason::SalesTaxElectionWithoutAmount,
                    "the §164(b)(5) sales-tax election is ON but `salt_sales_tax_amount` is $0, so Schedule \
                     A line 5a would be $0 and your state/local income taxes (withholding, estimates) drop \
                     out — enter the sales-tax amount, or turn the election off to deduct income taxes",
                );
            }
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

    // W-2 rows: box-12 allowlist + §402(g) deferral cap + box 8/10 + single-employer excess SS.
    let excess_ss_max = tbl.ss_wage_base * EMPLOYEE_OASDI_RATE; // §3101(a)/§6413(c)
                                                                // §402(g)(1) limits an INDIVIDUAL's elective deferrals — accumulate PER OWNER (each spouse on a joint
                                                                // return gets its own limit; review I1), refusing iff any one person exceeds it. Amounts are already
                                                                // guaranteed ≥ 0 by the negative screen above, so no per-entry clamp is needed.
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
        if w2.box4_ss_withheld > excess_ss_max {
            return refuse(
                RefuseReason::SingleEmployerExcessSs,
                "a single employer over-withheld Social Security — recover it from the employer (not creditable)",
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
            salt_cap: dec!(10000),
            kiddie_unearned_threshold: dec!(2600),
            elective_deferral_limit: dec!(23000),
            ftc_ceiling: dec!(300),
            qbi_ti_threshold_unmarried: dec!(191950),
            qbi_ti_threshold_married: dec!(383900),
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
        // FIVE always-live declarations are answered here so a computing fixture is not tripped by the
        // registry loop (§3.1 churn note); a test that wants one UNANSWERED re-blanks it explicitly.
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        ri.foreign_accounts = Some(false);
        ri.foreign_trust = Some(false);
        ri.sch1.hsa_activity = Some(false);
        ri.dual_status_alien = Some(false);
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
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        match id {
            QuestionId::DependentSpouse => r.filing_status = FilingStatus::Mfj, // live with no spouse Person (P8a I1)
            QuestionId::MfsSpouseItemizes => r.filing_status = FilingStatus::Mfs,
            QuestionId::MortgageAllUsedToBuyBuildImprove => {
                r.schedule_a = Some(ScheduleAInputs {
                    mortgage_interest_1098: dec!(9000),
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
            for other in FORM_QUESTIONS {
                if other.id != q.id && (other.live)(&r) {
                    (other.set)(&mut r, false);
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
            // Answering it (n) removes ITS unanswered reason (a value-refusal on a different axis may still
            // fire on some multi-part fixtures, so assert the specific reason is gone — not that all is well).
            (q.set)(&mut r, false);
            assert_ne!(
                reason(&r),
                Some(q.unanswered.clone()),
                "answered {:?} must no longer fire its unanswered reason",
                q.id
            );
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
        let sd = |r: &ReturnInputs| crate::tax::return_1040::standard_deduction(r, &p, 2024, Usd::ZERO);
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
            ..Default::default()
        });
        assert_eq!(reason(&ok), None, "a NAMED business must file");
    }

    /// A captured SSN that is not nine digits can never be printed on a form, so it fails HERE — at
    /// compute time, before any PDF is attempted (§3.4: an unprintable SSN is an uncomputable line).
    /// The refusal names WHO, never the digits: an SSN in an error string is a PII leak.
    #[test]
    fn a_malformed_ssn_refuses_at_compute_time() {
        let mut r = ri();
        r.header.taxpayer.ssn = "12345".into(); // five digits — not an SSN
        assert!(matches!(reason(&r), Some(RefuseReason::SsnMalformed(who)) if who == "taxpayer"));

        let mut r = ri();
        r.filing_status = FilingStatus::Mfj;
        r.header.spouse = Some(crate::tax::return_inputs::Person {
            ssn: "123-45-678X".into(),
            ..Default::default()
        });
        assert!(matches!(reason(&r), Some(RefuseReason::SsnMalformed(who)) if who == "spouse"));

        let mut r = ri();
        r.header
            .dependents
            .push(crate::tax::return_inputs::Dependent {
                name: "Sam Doe".into(),
                ssn: "1234567890".into(), // ten digits
                ..Default::default()
            });
        assert!(
            matches!(reason(&r), Some(RefuseReason::SsnMalformed(who)) if who.contains("Sam Doe"))
        );
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

    #[test]
    fn single_employer_excess_ss_refuses() {
        let mut r = ri();
        // One employer withheld more than 6.2% × 176,100 = 10,918.20.
        r.w2s.push(W2 {
            box4_ss_withheld: dec!(11000),
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::SingleEmployerExcessSs));
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
        use crate::tax::return_inputs::ScheduleAInputs;
        let mut r = ri();
        r.schedule_a = Some(ScheduleAInputs {
            salt_use_sales_tax: Some(true),
            salt_sales_tax_amount: Usd::ZERO,
            salt_state_estimated_payments: dec!(5000), // income-tax SALT that the election would ZERO out
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::SalesTaxElectionWithoutAmount));
        // With a sales-tax amount → no collapse, no refusal.
        r.schedule_a.as_mut().unwrap().salt_sales_tax_amount = dec!(3000);
        assert_ne!(reason(&r), Some(RefuseReason::SalesTaxElectionWithoutAmount));
        // Election on, $0 amount, but NO income-tax SALT to lose → nothing collapses, so NOT this refusal.
        let mut r2 = ri();
        r2.schedule_a = Some(ScheduleAInputs {
            salt_use_sales_tax: Some(true),
            salt_sales_tax_amount: Usd::ZERO,
            ..Default::default()
        });
        assert_ne!(reason(&r2), Some(RefuseReason::SalesTaxElectionWithoutAmount));
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
