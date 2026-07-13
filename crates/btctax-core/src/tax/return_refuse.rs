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
use crate::tax::return_1040::schedule_b_part3_unanswered;
use crate::tax::return_inputs::{
    Box12Entry, CharitableCarryItem, CharitableGift, Form1099Div, Form1099G, Form1099Int, Owner,
    Payments, QbiInputs, ReturnInputs, Schedule1Inputs, ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::tables::{FullReturnParams, TaxTable};
use crate::tax::types::{Carryforward, FilingStatus};
use rust_decimal_macros::dec;

/// §3101(a) employee OASDI (Social Security) tax rate — the excess-SS credit is computed against this
/// (6.2%), NOT the 12.4% combined SE rate. Statutory.
const EMPLOYEE_OASDI_RATE: Usd = dec!(0.062);

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
    /// Foreign tax > the §904(j) $300/$600 no-Form-1116 ceiling.
    ForeignTaxOverCeiling,
    /// A single employer over-withheld Social Security (not creditable — recover from the employer).
    SingleEmployerExcessSs,
    /// Schedule 1 line 13 HSA present → Form 8889 mandatory.
    HsaPresent,
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
    /// A claimable-as-dependent filer with unearned income over the §1(g) kiddie-tax threshold → Form 8615
    /// (the child's-rate `qdcgt_line16` would understate; the parent's rate is required — C1/F2).
    KiddieTax,
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
        if neg(*box1_wages) { return Some("W-2 box 1 wages"); }
        if neg(*box2_fed_withheld) { return Some("W-2 box 2 federal withholding"); }
        if neg(*box3_ss_wages) { return Some("W-2 box 3 Social Security wages"); }
        if neg(*box4_ss_withheld) { return Some("W-2 box 4 Social Security withholding"); }
        if neg(*box5_medicare_wages) { return Some("W-2 box 5 Medicare wages"); }
        if neg(*box6_medicare_withheld) { return Some("W-2 box 6 Medicare withholding"); }
        if neg(*box7_ss_tips) { return Some("W-2 box 7 Social Security tips"); }
        if neg(*box17_state_tax_withheld) { return Some("W-2 box 17 state tax withheld"); }
        if neg(*box19_local_tax) { return Some("W-2 box 19 local tax"); }
        if neg(*box8_allocated_tips) { return Some("W-2 box 8 allocated tips"); }
        if neg(*box10_dependent_care) { return Some("W-2 box 10 dependent-care benefits"); }
        for e in box12 {
            let Box12Entry { code: _, amount } = e;
            if neg(*amount) { return Some("W-2 box 12 amount"); }
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
        if neg(*box1_interest) { return Some("1099-INT box 1 interest"); }
        if neg(*box2_early_withdrawal_penalty) { return Some("1099-INT box 2 early-withdrawal penalty"); }
        if neg(*box3_treasury_interest) { return Some("1099-INT box 3 Treasury interest"); }
        if neg(*box4_fed_withheld) { return Some("1099-INT box 4 federal withholding"); }
        if neg(*box6_foreign_tax) { return Some("1099-INT box 6 foreign tax"); }
        if neg(*box8_tax_exempt_interest) { return Some("1099-INT box 8 tax-exempt interest"); }
        if neg(*box9_private_activity_bond_amt) { return Some("1099-INT box 9 private-activity-bond interest"); }
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
        if neg(*box1a_ordinary) { return Some("1099-DIV box 1a ordinary dividends"); }
        if neg(*box1b_qualified) { return Some("1099-DIV box 1b qualified dividends"); }
        if neg(*box2a_capgain_distr) { return Some("1099-DIV box 2a capital-gain distributions"); }
        if neg(*box2b_unrecap_1250) { return Some("1099-DIV box 2b unrecaptured §1250 gain"); }
        if neg(*box2c_section_1202) { return Some("1099-DIV box 2c §1202 gain"); }
        if neg(*box2d_collectibles_28) { return Some("1099-DIV box 2d collectibles (28%) gain"); }
        if neg(*box4_fed_withheld) { return Some("1099-DIV box 4 federal withholding"); }
        if neg(*box5_section_199a) { return Some("1099-DIV box 5 §199A dividends"); }
        if neg(*box7_foreign_tax) { return Some("1099-DIV box 7 foreign tax"); }
        if neg(*box12_exempt_interest_dividends) { return Some("1099-DIV box 12 exempt-interest dividends"); }
        if neg(*box13_private_activity_amt) { return Some("1099-DIV box 13 private-activity-bond dividends"); }
    }
    for g in g_1099 {
        let Form1099G { payer: _, box1_unemployment, box4_fed_withheld } = g;
        if neg(*box1_unemployment) { return Some("1099-G box 1 unemployment compensation"); }
        if neg(*box4_fed_withheld) { return Some("1099-G box 4 federal withholding"); }
    }
    if let Some(c) = schedule_c {
        let ScheduleCInputs {
            owner: _,
            business_description: _,
            naics_code: _,
            accounting_method: _,
            expenses,
        } = c;
        if neg(*expenses) { return Some("Schedule C expenses"); }
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
            charitable,
        } = a;
        if neg(*medical) { return Some("Schedule A medical expenses"); }
        if neg(*salt_sales_tax_amount) { return Some("Schedule A sales-tax amount"); }
        if neg(*salt_state_estimated_payments) { return Some("Schedule A state estimated payments"); }
        if neg(*salt_prior_year_balance_paid) { return Some("Schedule A prior-year balance paid"); }
        if neg(*salt_real_estate) { return Some("Schedule A real-estate taxes"); }
        if neg(*salt_personal_property) { return Some("Schedule A personal-property taxes"); }
        if neg(*mortgage_interest_1098) { return Some("Schedule A mortgage interest"); }
        for gift in charitable {
            let CharitableGift { class: _, amount } = gift;
            if neg(*amount) { return Some("Schedule A charitable gift amount"); }
        }
    }
    for item in charitable_carryover_in {
        let CharitableCarryItem { class: _, amount, origin_year: _ } = item;
        if neg(*amount) { return Some("charitable carryover amount"); }
    }
    let Schedule1Inputs {
        state_refund_taxable,
        student_loan_interest_paid,
        ira_deduction_claimed,
        hsa_present: _,
    } = sch1;
    if neg(*state_refund_taxable) { return Some("Schedule 1 taxable state refund"); }
    if neg(*student_loan_interest_paid) { return Some("Schedule 1 student-loan interest"); }
    if neg(*ira_deduction_claimed) { return Some("Schedule 1 IRA deduction"); }
    let Payments { estimated_tax_payments, extension_payment, other_withholding } = payments;
    if neg(*estimated_tax_payments) { return Some("estimated tax payments"); }
    if neg(*extension_payment) { return Some("extension payment"); }
    if neg(*other_withholding) { return Some("other withholding"); }
    let QbiInputs { reit_ptp_carryforward_in } = qbi;
    if neg(*reit_ptp_carryforward_in) { return Some("QBI REIT/PTP carryforward"); }
    let Carryforward { short, long } = capital_loss_carryforward_in;
    if neg(*short) { return Some("short-term capital-loss carryforward"); }
    if neg(*long) { return Some("long-term capital-loss carryforward"); }
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

    // (c) foreign trust → Form 3520.
    if ri.foreign_trust == Some(true) {
        return refuse(
            RefuseReason::ForeignTrust,
            "a foreign trust requires Form 3520, which is out of scope for v1",
        );
    }

    // Schedule B Part III (7a/8) must be answered when Schedule B files — a `None` tri-state fails loud
    // rather than guess a foreign-account/-trust disclosure (SPEC §7.1, plan P2 task 4 / P2-I1).
    if schedule_b_part3_unanswered(ri) {
        return refuse(
            RefuseReason::ScheduleBPart3Unanswered,
            "Schedule B is required (interest/dividends > $1,500 or a foreign account) but its Part III \
             foreign-account/foreign-trust questions are unanswered — set `foreign_accounts`/`foreign_trust`",
        );
    }

    // Schedule A §164(b)(5) SALT: a sales-tax amount with the election OFF is an input error — fail loud
    // rather than silently drop it (R3-M9).
    if let Some(a) = &ri.schedule_a {
        if a.salt_sales_tax_amount > Usd::ZERO && !a.salt_use_sales_tax {
            return refuse(
                RefuseReason::SaltSalesTaxWithoutElection,
                "a Schedule A sales-tax amount is set but the §164(b)(5) sales-tax election is off — turn \
                 the election on (5a = sales tax) or clear `salt_sales_tax_amount`",
            );
        }
    }

    // §63(c)(6): an MFS return must state whether the spouse itemizes (it forces this filer's std/itemize
    // choice); a `None` tri-state is fail-loud, never a silent assumption (G15).
    if ri.filing_status == FilingStatus::Mfs && ri.mfs_spouse_itemizes.is_none() {
        return refuse(
            RefuseReason::MfsSpouseItemizeUnknown,
            "a married-filing-separately return must state whether the spouse itemizes (§63(c)(6)) — set \
             `mfs_spouse_itemizes`",
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

    // Schedule 1 minimal surface: HSA and any claimed IRA deduction refuse in v1.
    if ri.sch1.hsa_present {
        return refuse(
            RefuseReason::HsaPresent,
            "an HSA requires Form 8889 — out of scope for v1",
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
            student_loan_phaseout_unmarried: (dec!(80000), dec!(95000)),
            student_loan_phaseout_married: (dec!(165000), dec!(195000)),
        }
    }
    fn tbl() -> TaxTable {
        crate::tax::tables::synthetic_table(2024) // ss_wage_base = 176,100 (synthetic); MAX = 10,918.20
    }
    fn ri() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        }
    }
    fn reason(ri: &ReturnInputs) -> Option<RefuseReason> {
        screen_inputs(ri, &tbl(), &params()).map(|r| r.reason)
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
        assert_eq!(reason(&r), Some(RefuseReason::UnsupportedBox12Code("K".into())));
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
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(15000) }],
            ..Default::default()
        });
        r.w2s.push(W2 {
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(10000) }],
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::ExcessElectiveDeferral));
        // MFJ dual-earner: $15k taxpayer + $15k spouse — each under $23k → NO refuse (review I1).
        let mut ok = ri();
        ok.filing_status = FilingStatus::Mfj;
        ok.w2s.push(W2 {
            owner: Owner::Taxpayer,
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(15000) }],
            ..Default::default()
        });
        ok.w2s.push(W2 {
            owner: Owner::Spouse,
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(15000) }],
            ..Default::default()
        });
        assert_eq!(reason(&ok), None);
    }

    #[test]
    fn box8_box10_refuse() {
        let mut a = ri();
        a.w2s.push(W2 { box8_allocated_tips: dec!(500), ..Default::default() });
        assert_eq!(reason(&a), Some(RefuseReason::AllocatedTips));
        let mut b = ri();
        b.w2s.push(W2 { box10_dependent_care: dec!(5000), ..Default::default() });
        assert_eq!(reason(&b), Some(RefuseReason::DependentCareBenefit));
    }

    #[test]
    fn single_employer_excess_ss_refuses() {
        let mut r = ri();
        // One employer withheld more than 6.2% × 176,100 = 10,918.20.
        r.w2s.push(W2 { box4_ss_withheld: dec!(11000), ..Default::default() });
        assert_eq!(reason(&r), Some(RefuseReason::SingleEmployerExcessSs));
    }

    #[test]
    fn amt_preference_and_special_gains_refuse() {
        let mut a = ri();
        a.int_1099.push(Form1099Int { box9_private_activity_bond_amt: dec!(10), ..Default::default() });
        assert_eq!(reason(&a), Some(RefuseReason::PrivateActivityBondAmt));
        let mut b = ri();
        b.div_1099.push(Form1099Div { box2d_collectibles_28: dec!(50), ..Default::default() });
        assert_eq!(reason(&b), Some(RefuseReason::UnrecapturedOrSpecialRateGain));
    }

    #[test]
    fn foreign_tax_over_ceiling_refuses() {
        // Single: $301 > $300 ceiling.
        let mut r = ri();
        r.div_1099.push(Form1099Div { box7_foreign_tax: dec!(301), ..Default::default() });
        assert_eq!(reason(&r), Some(RefuseReason::ForeignTaxOverCeiling));
        // MFJ ceiling is doubled ($600): $301 is fine.
        let mut mfj = r.clone();
        mfj.filing_status = FilingStatus::Mfj;
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
        r.div_1099.push(Form1099Div { box7_foreign_tax: dec!(500), ..Default::default() });
        r.int_1099.push(Form1099Int { box6_foreign_tax: dec!(-250), ..Default::default() });
        assert_eq!(
            reason(&r),
            Some(RefuseReason::NegativeAmount("1099-INT box 6 foreign tax".into()))
        );
        // Same shape for a negative elective deferral (the old M4 vector) and a plain negative wage.
        let mut d = ri();
        d.w2s.push(W2 {
            box12: vec![
                Box12Entry { code: "D".into(), amount: dec!(30000) },
                Box12Entry { code: "D".into(), amount: dec!(-10000) },
            ],
            ..Default::default()
        });
        assert_eq!(
            reason(&d),
            Some(RefuseReason::NegativeAmount("W-2 box 12 amount".into()))
        );
        let mut w = ri();
        w.w2s.push(W2 { box1_wages: dec!(-1), ..Default::default() });
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
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(15000) }],
            ..Default::default()
        });
        single.w2s.push(W2 {
            owner: Owner::Spouse,
            box12: vec![Box12Entry { code: "D".into(), amount: dec!(15000) }],
            ..Default::default()
        });
        assert_eq!(reason(&single), Some(RefuseReason::SpouseOwnerWithoutJointReturn));
        // A spouse-owned Schedule C on a non-joint return also refuses.
        let mut hoh = ri();
        hoh.filing_status = FilingStatus::HoH;
        hoh.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            owner: Owner::Spouse,
            ..Default::default()
        });
        assert_eq!(reason(&hoh), Some(RefuseReason::SpouseOwnerWithoutJointReturn));
        // The SAME split on a joint return is legitimate (two earners) → no spouse-owner refusal.
        let mut mfj = single.clone();
        mfj.filing_status = FilingStatus::Mfj;
        assert_eq!(reason(&mfj), None);
    }

    #[test]
    fn schedule_b_part3_unanswered_refuses() {
        // Files ($2,000 interest > $1,500) but Part III foreign-account/-trust questions unanswered (None).
        let mut r = ri();
        r.int_1099.push(Form1099Int {
            box1_interest: dec!(2000),
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::ScheduleBPart3Unanswered));
        // Answer BOTH 7a and 8 → no refusal.
        r.foreign_accounts = Some(false);
        r.foreign_trust = Some(false);
        assert_eq!(reason(&r), None);
        // Line 8 (foreign trust) left unanswered while filing → still fail-loud.
        r.foreign_trust = None;
        assert_eq!(reason(&r), Some(RefuseReason::ScheduleBPart3Unanswered));
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
            salt_use_sales_tax: false, // amount set but election OFF → input error
            ..Default::default()
        });
        assert_eq!(reason(&r), Some(RefuseReason::SaltSalesTaxWithoutElection));
        // Election ON → no refusal.
        r.schedule_a.as_mut().unwrap().salt_use_sales_tax = true;
        assert_eq!(reason(&r), None);
    }

    #[test]
    fn hsa_and_ira_refuse() {
        let mut a = ri();
        a.sch1.hsa_present = true;
        assert_eq!(reason(&a), Some(RefuseReason::HsaPresent));
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
}
