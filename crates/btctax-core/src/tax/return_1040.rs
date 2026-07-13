//! Full-return v1 **absolute 1040 assembly** (Phase 2+). This module builds the *absolute* filed return
//! from `ReturnInputs` + the projected ledger, and — the load-bearing Phase-2 piece — derives the FROZEN
//! [`TaxProfile`] the crypto-delta engine consumes.
//!
//! **The frozen seam (SPEC §5 / deep/02).** Two AGI notions coexist and must never be conflated:
//! - [`derive_tax_profile`] populates the frozen `TaxProfile` scalars from **NON-crypto line items only**.
//!   `ReturnInputs` holds no crypto (crypto lives in the ledger `state`), so the exclusion is *structural*:
//!   this function cannot see, and therefore cannot double-count, any crypto figure. The frozen engine
//!   (`compute.rs`) adds the crypto AGI delta itself (`compute.rs:339-342` `bottom_with`), so the profile
//!   must exclude it (`types.rs:34-36`).
//! - The *absolute* WITH-crypto 1040 (the filed return, added in a later P2 increment) re-combines the
//!   non-crypto lines with the ledger's crypto figures **itself**, via the shared primitives (`net_1222`,
//!   `ordinary_tax_on`, `preferential_tax`) — never by un-delta-ing `compute_tax_year`.
//!
//! Additive per SPEC §2: `compute.rs` / `types.rs` / `se.rs` stay byte-frozen; this file only reads them.
use crate::conventions::{round_dollar, Usd};
use crate::forms::schedule_d;
use crate::state::LedgerState;
use crate::tax::compute::net_1222;
use crate::tax::return_inputs::{Owner, Person, ReturnInputs};
use crate::tax::return_refuse::{Refusal, RefuseReason};
use crate::tax::se::se_net_income;
use crate::tax::tables::{loss_limit, FullReturnParams};
use crate::tax::types::{FilingStatus, TaxProfile};
use crate::IncomeKind;
use rust_decimal_macros::dec;
use time::{Date, Month};

// ── §63 standard deduction (Phase 3 task 1) ──────────────────────────────────────────────────────

/// Whether `dob` makes a person **aged (65+)** for the §63(f) additional standard deduction in tax `year`.
/// IRS rule (Pub 501): 65 if born **on or before January 1 of `year − 64`** (turned 65 by the Jan-1-after-
/// year-end test). A `None` DOB is "not established" → NOT counted as aged: the conservative, fail-closed
/// direction — never grant an unsubstantiated deduction, and never silently assume a birthdate
/// (burns down the `dob-option-pin` follow-up; §4.2 / review r1-M3).
fn is_aged(dob: Option<Date>, year: i32) -> bool {
    let Some(d) = dob else {
        return false;
    };
    Date::from_calendar_date(year - 64, Month::January, 1).is_ok_and(|cutoff| d <= cutoff)
}

/// The number of §63(f) aged/blind "boxes" a person contributes (0, 1, or 2): +1 if 65+ (by DOB), +1 if
/// blind (an explicit flag — not DOB-derivable, §4.2).
fn aged_blind_boxes(p: &Person, year: i32) -> u32 {
    u32::from(is_aged(p.date_of_birth, year)) + u32::from(p.blind)
}

/// §63(f) additional-standard-deduction rate is the **married** amount for MFJ/MFS/QSS (a "surviving
/// spouse" is in the joint bucket here, like `Qss → Mfj` for the basic deduction), **unmarried** for
/// Single/HoH.
fn uses_married_aged_blind_rate(status: FilingStatus) -> bool {
    matches!(
        status,
        FilingStatus::Mfj | FilingStatus::Mfs | FilingStatus::Qss
    )
}

/// §63(c) **standard deduction**: the basic amount (or the §63(c)(5) dependent floor when the filer can be
/// claimed as a dependent) PLUS the §63(f) aged/blind additions.
///
/// `dependent_earned_income` matters ONLY for a can-be-claimed-as-dependent filer (§63(c)(5): the base is
/// capped at the basic std, floored at `max($1,300, earned + $450)`). The **derivation** passes the
/// NON-crypto earned income (wages); the absolute return passes with-crypto earned (wages + Sch C net −
/// ½-SE) — a documented delta-vs-absolute divergence (§6) only in the rare dependent-filer case.
///
/// §63(f) boxes: the taxpayer always, plus the spouse **on a joint (MFJ) return**. (On MFS the rare
/// no-income-spouse box is conservatively not counted — never over-granting.)
pub fn standard_deduction(
    ri: &ReturnInputs,
    params: &FullReturnParams,
    year: i32,
    dependent_earned_income: Usd,
) -> Usd {
    let status = ri.filing_status;
    let basic = params.std_deduction_for(status);
    let base = if ri.header.can_be_claimed_as_dependent_taxpayer {
        // §63(c)(5): min(basic, max($1,300, earned + $450)).
        basic.min(
            (dependent_earned_income + params.dependent_std_earned_addon)
                .max(params.dependent_std_floor),
        )
    } else {
        basic
    };

    let mut boxes = aged_blind_boxes(&ri.header.taxpayer, year);
    if status == FilingStatus::Mfj {
        if let Some(sp) = &ri.header.spouse {
            boxes += aged_blind_boxes(sp, year);
        }
    }
    let per_box = if uses_married_aged_blind_rate(status) {
        params.std_aged_blind_married
    } else {
        params.std_aged_blind_unmarried
    };

    base + per_box * Usd::from(boxes)
}

// ── Schedule A itemized deduction (Phase 3 task 2) ───────────────────────────────────────────────

/// The §164(b)(5) SALT line 5a election: `true` (sales-tax path) → `salt_sales_tax_amount` ONLY; `false`
/// (income-tax path) → W-2 state/local withholding + state estimated payments + prior-year balance paid.
/// (A nonzero `salt_sales_tax_amount` with the election OFF is refused upstream — R3-M9.)
fn salt_line_5a(ri: &ReturnInputs, a: &crate::tax::return_inputs::ScheduleAInputs) -> Usd {
    if a.salt_use_sales_tax {
        a.salt_sales_tax_amount
    } else {
        let w2_wh: Usd = ri
            .w2s
            .iter()
            .map(|w| w.box17_state_tax_withheld + w.box19_local_tax)
            .sum();
        w2_wh + a.salt_state_estimated_payments + a.salt_prior_year_balance_paid
    }
}

/// The **Schedule A itemized deduction total** (line 17) at `agi`, given the already-§170(b)-limited
/// charitable total (line 14 from [`crate::tax::charitable::apply_170b`]). `None` when the filer has no
/// Schedule A (takes the standard deduction). Medical floor 7.5%·AGI (line 4); SALT §164(b)(5) either/or
/// capped at $10k ($5k MFS) (line 5e); mortgage interest (line 8a); charitable (line 14).
///
/// `agi` is the caller's AGI: the derivation passes NON-crypto AGI (and non-crypto charitable); the
/// absolute return passes with-crypto AGI (+ crypto donations) — a documented delta-vs-absolute divergence
/// (§6) whenever an AGI-sensitive line (medical floor, charitable ceiling) binds.
pub fn schedule_a_deduction(
    ri: &ReturnInputs,
    agi: Usd,
    charitable_allowed: Usd,
    params: &FullReturnParams,
) -> Option<Usd> {
    let a = ri.schedule_a.as_ref()?;
    // Line 4 — medical/dental over the 7.5%-of-AGI floor.
    let medical = (a.medical - dec!(0.075) * agi).max(Usd::ZERO);
    // Line 5e — SALT, §164(b)(5) either/or, capped at $10,000 ($5,000 MFS).
    let salt_5d = salt_line_5a(ri, a) + a.salt_real_estate + a.salt_personal_property;
    let salt_cap = if ri.filing_status == FilingStatus::Mfs {
        params.salt_cap / dec!(2)
    } else {
        params.salt_cap
    };
    let salt_5e = salt_5d.min(salt_cap);
    // Line 8a — home-mortgage interest (points/8b are refuse-or-advise in P6).
    let mortgage = a.mortgage_interest_1098;
    Some(medical + salt_5e + mortgage + charitable_allowed)
}

/// §63(e)/(c)(6) deduction CHOICE: `max(standard, itemized)` by default; `ForceItemize` honors §63(e)
/// (itemize even if smaller); **MFS with an itemizing spouse** forces this filer's standard deduction to
/// $0 (§63(c)(6) — the spouses must agree). `itemized` is `None` when there is no Schedule A.
fn choose_deduction(ri: &ReturnInputs, standard: Usd, itemized: Option<Usd>) -> Usd {
    use crate::tax::return_inputs::ItemizeElection;
    let itemized = itemized.unwrap_or(Usd::ZERO);
    // §63(c)(6): an MFS filer whose spouse itemizes gets NO standard deduction (a `None` tri-state on MFS
    // is refused upstream — G15).
    let standard = if ri.filing_status == FilingStatus::Mfs && ri.mfs_spouse_itemizes == Some(true) {
        Usd::ZERO
    } else {
        standard
    };
    match ri.itemize_election {
        ItemizeElection::ForceItemize => itemized,
        ItemizeElection::Auto => standard.max(itemized),
    }
}

// ── Non-crypto income-line sums (shared by the derivation, the refuse screen, and the absolute 1040) ──
fn sum_wages(ri: &ReturnInputs) -> Usd {
    ri.w2s.iter().map(|w| w.box1_wages).sum()
}
/// 1040 2b taxable interest = box 1 + box 3 (Treasury); box 3 is NOT a subset of box 1.
fn sum_taxable_interest(ri: &ReturnInputs) -> Usd {
    ri.int_1099
        .iter()
        .map(|i| i.box1_interest + i.box3_treasury_interest)
        .sum()
}
/// 1040 3b ordinary dividends = Σ box 1a (ALREADY includes box 1b qualified — "strip once").
fn sum_ordinary_dividends(ri: &ReturnInputs) -> Usd {
    ri.div_1099.iter().map(|d| d.box1a_ordinary).sum()
}
/// 1040 3a qualified dividends = Σ box 1b (the preferential split ONLY — never added to income again).
fn sum_qualified_dividends(ri: &ReturnInputs) -> Usd {
    ri.div_1099.iter().map(|d| d.box1b_qualified).sum()
}
/// Σ box 2a capital-gain distributions (LT character; enters AGI once via Sch D → 1040 L7).
fn sum_cap_gain_distr(ri: &ReturnInputs) -> Usd {
    ri.div_1099.iter().map(|d| d.box2a_capgain_distr).sum()
}
/// Sch 1 L7 unemployment compensation = Σ 1099-G box 1.
fn sum_unemployment(ri: &ReturnInputs) -> Usd {
    ri.g_1099.iter().map(|g| g.box1_unemployment).sum()
}

/// The crypto income figures for `year` from the projected ledger (the WITH-crypto side of the return).
struct CryptoIncome {
    /// Σ business SE-eligible crypto income (kind ≠ Interest) → Schedule C gross (deep/02 / `se_net_income`).
    business_se_gross: Usd,
    /// Σ business-flagged crypto `Interest` → has no clean v1 home (refuses, R3-I3).
    business_interest: Usd,
    /// Σ non-business crypto ordinary income (any kind) → Sch 1 L8v (hobby rewards + lending interest).
    nonbusiness_ordinary: Usd,
}

fn crypto_income(state: &LedgerState, year: i32) -> CryptoIncome {
    let mut business_interest = Usd::ZERO;
    let mut nonbusiness_ordinary = Usd::ZERO;
    for i in state
        .income_recognized
        .iter()
        .filter(|i| i.recognized_at.year() == year)
    {
        if i.business {
            if i.kind == IncomeKind::Interest {
                business_interest += i.usd_fmv;
            }
        } else {
            nonbusiness_ordinary += i.usd_fmv;
        }
    }
    CryptoIncome {
        business_se_gross: se_net_income(state, year), // canonical business SE-eligible sum
        business_interest,
        nonbusiness_ordinary,
    }
}

/// The amount reaching **1040 L7** (capital gain or loss) for `year`: crypto Schedule D nets + box-2a
/// capital-gain distributions, run through §1222 within-character netting + the §1211 loss limit. In a
/// gain year this is the full net gain; in a loss year it is the −$3,000/−$1,500-MFS limited loss.
fn capital_gain_line7(ri: &ReturnInputs, state: &LedgerState, year: i32, status: FilingStatus) -> Usd {
    let sd = schedule_d(state, year); // raw crypto ST/LT nets (traverses state.disposals)
    let cf = ri.capital_loss_carryforward_in;
    let net = net_1222(
        sd.st.gain,
        sd.lt.gain,
        sum_cap_gain_distr(ri), // box 2a is LT-character "other" capital gain
        cf.short,
        cf.long,
        loss_limit(status),
    );
    net.ordinary_gain + net.preferential_gain - net.loss_deduction
}

fn refusal(reason: RefuseReason, detail: &str) -> Option<Refusal> {
    Some(Refusal {
        reason,
        detail: detail.to_string(),
    })
}

/// Screen the **compute-dependent** refuse rows (SPEC §4.10) — those that need the assembled income /
/// ledger, not just `ReturnInputs`. Returns the FIRST [`Refusal`], or `None`. Complements
/// [`crate::tax::return_refuse::screen_inputs`] (the input-screenable rows); both must pass before a
/// full-return year is computed (fail-closed).
pub fn screen_compute_dependent(
    ri: &ReturnInputs,
    state: &LedgerState,
    year: i32,
    params: &FullReturnParams,
) -> Option<Refusal> {
    let crypto = crypto_income(state, year);

    // Business-flagged crypto Interest has no clean v1 home (excluded from SE, not NIIT-sheltered).
    if crypto.business_interest > Usd::ZERO {
        return refusal(
            RefuseReason::BusinessInterestIncome,
            "business-flagged crypto interest income is excluded from SE tax (§1402(a)(2)) but not from NIIT — unsupported in v1",
        );
    }

    // Schedule C net = business SE gross − expenses. No Schedule C but business income ⇒ fail loud; loss ⇒ refuse.
    let sch_c_net = match &ri.schedule_c {
        None => {
            if crypto.business_se_gross > Usd::ZERO {
                return refusal(
                    RefuseReason::BusinessIncomeWithoutScheduleC,
                    "the ledger has SE-eligible business crypto income but no Schedule C was provided (`income import`); owner and description are required",
                );
            }
            Usd::ZERO
        }
        Some(sc) => {
            let net = crypto.business_se_gross - sc.expenses;
            if net < Usd::ZERO {
                return refusal(
                    RefuseReason::ScheduleCLoss,
                    "Schedule C net profit is negative (a loss) — §465 at-risk substantiation is out of scope for v1",
                );
            }
            net
        }
    };

    // §1(g) Form 8615 kiddie tax: a claimable-as-dependent filer with unearned income over the threshold.
    // unearned = gross income − earned income (wages + Schedule C net) — SPEC F2. This component-sum OMITS
    // the Sch-1 adjustments (early-withdrawal penalty, student-loan deduction) that Form 8615's true
    // `AGI − earned` would net out, so `unearned` here can only be TOO HIGH ⇒ it can only OVER-refuse
    // (conservative / fail-closed — review M4). Do NOT "fix" by subtracting the adjustments without
    // preserving that direction: an under-count would let a real kiddie return slip through at the child's
    // rate (an understatement). A capital LOSS correctly lowers unearned (`capital_gain_line7` is the
    // §1211-limited L7, which the Form 8615 worksheet also uses) — that is not an under-refuse.
    if ri.header.can_be_claimed_as_dependent_taxpayer {
        let unearned = sum_taxable_interest(ri)
            + sum_ordinary_dividends(ri)
            + capital_gain_line7(ri, state, year, ri.filing_status)
            + ri.sch1.state_refund_taxable
            + sum_unemployment(ri)
            + crypto.nonbusiness_ordinary;
        let _ = sch_c_net; // earned income (wages + sch_c_net) is excluded from `unearned` by construction
        if unearned > params.kiddie_unearned_threshold {
            return refusal(
                RefuseReason::KiddieTax,
                "a claimable-as-dependent filer with unearned income over the §1(g) threshold needs Form 8615 (parent's-rate tax) — out of scope for v1",
            );
        }
    }

    None
}

/// §221 student-loan-interest deduction (Sch 1 L21): `min(paid, $2,500)` phased out linearly over the
/// filing status's MAGI range (**MFS ⇒ $0**, §221(e)(2)). `magi` is the AGI **before** this deduction.
///
/// In [`derive_tax_profile`] the `magi` passed is the **non-crypto** AGI-before-L21 (the delta baseline);
/// the absolute return uses the with-crypto AGI — a deliberate, documented delta-vs-absolute divergence
/// (SPEC §6), since the frozen engine fixes the deduction at derivation time.
///
/// The IRS worksheet says "round [the ratio] to at least three places"; using the exact ratio satisfies
/// that (∞ places) and we `round_dollar` the final amount per the global half-up policy (SPEC §3.1).
pub fn student_loan_deduction(
    paid: Usd,
    magi: Usd,
    status: FilingStatus,
    params: &FullReturnParams,
) -> Usd {
    let cap = paid.min(dec!(2500));
    if cap <= Usd::ZERO {
        return Usd::ZERO;
    }
    match params.student_loan_phaseout(status) {
        None => Usd::ZERO, // MFS — no deduction
        Some((lo, hi)) => {
            if magi <= lo {
                cap
            } else if magi >= hi {
                Usd::ZERO
            } else {
                let ratio = (magi - lo) / (hi - lo);
                round_dollar(cap * (Usd::ONE - ratio))
            }
        }
    }
}

/// Derive the FROZEN [`TaxProfile`] (crypto-delta-engine input) from the **non-crypto** `ReturnInputs`
/// line items for `year`'s `params` (SPEC §5 stages 1–2, deep/02 §1 Worked Example 1).
///
/// Crypto is **excluded structurally** — `ReturnInputs` carries none; the engine adds the crypto delta on
/// top. **P3:** the deduction is now the FULL §63 standard deduction (basic + §63(f) aged/blind + the
/// dependent floor, with NON-crypto earned income = wages); Schedule A (the `max(std, itemized)`) and QBI
/// land later in P3/P4. `magi_excluding_crypto = AGI` exactly (no §911/CFC/PFIC in the model — deep/02 C1).
pub fn derive_tax_profile(ri: &ReturnInputs, params: &FullReturnParams, year: i32) -> TaxProfile {
    let status = ri.filing_status;

    // ── Income (non-crypto) ──────────────────────────────────────────────────────────────────────
    let wages = sum_wages(ri);
    let taxable_int = sum_taxable_interest(ri);
    let ord_div = sum_ordinary_dividends(ri);
    let qual_div = sum_qualified_dividends(ri);
    let cap_gain_distr = sum_cap_gain_distr(ri); // box 2a → Sch D L13 → 1040 L7 (LT character)

    // Sch 1 Part I additional income (non-crypto): L1 taxable state refund + L7 Σ unemployment.
    // (L3 Schedule C and L8v digital-asset income are CRYPTO → excluded from the frozen profile.)
    let sch1_income = ri.sch1.state_refund_taxable + sum_unemployment(ri);

    // Sch 1 Part II adjustments (non-crypto): L18 early-withdrawal penalty + L21 student-loan.
    // (L15 ½-SE is crypto-Schedule-C-driven → excluded here.)
    let early_wd: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box2_early_withdrawal_penalty)
        .sum();
    let income_total = wages + taxable_int + ord_div + cap_gain_distr + sch1_income;
    let agi_before_student_loan = income_total - early_wd;
    let student_loan = student_loan_deduction(
        ri.sch1.student_loan_interest_paid,
        agi_before_student_loan,
        status,
        params,
    );
    let adjustments = early_wd + student_loan;

    // ── AGI, deduction, taxable income ────────────────────────────────────────────────────────────
    let agi = income_total - adjustments; // 1040 L11 (non-crypto)
    // Deduction = max(full §63 standard, NON-crypto Schedule A) — P3 tasks 1–3. The derived Schedule A uses
    // the NON-crypto charitable (user gifts + carryover, §170(b)-limited at non-crypto AGI); crypto donations
    // belong to the absolute return, not the frozen delta (§6 divergence). The dependent-floor + charitable
    // ceilings key off this non-crypto AGI.
    let full_std = standard_deduction(ri, params, year, wages);
    let charitable = ri.schedule_a.as_ref().map_or(Usd::ZERO, |a| {
        crate::tax::charitable::apply_170b(agi, &a.charitable, &ri.charitable_carryover_in, year).allowed
    });
    let itemized = schedule_a_deduction(ri, agi, charitable, params);
    let deduction = choose_deduction(ri, full_std, itemized);
    let taxable_income = (agi - deduction).max(Usd::ZERO); // 1040 L15 (non-crypto)
    // Strip the preferential slice (qualified div + LT cap-gain distr) EXACTLY ONCE — the engine re-adds
    // it on top of the ordinary bottom via `other_net_capital_gain` + the QD channel (deep/02 §1.4).
    // KNOWN APPROXIMATION (audit-M2 / review M1, → P3 FOLLOWUP): when `TI < qd + cap_gain_distr` (low
    // ordinary income + large preferential income — e.g. a retiree), the `.max(0)` floors the ordinary
    // base to 0 while the FULL pref slice still reaches the frozen engine (which stacks `qd + pref_gain`
    // with no min-against-TI cap). The reconstructed TI is then ≥ the true TI, so the delta/planning
    // number can only OVERSTATE, never understate (conservative). Exact handling (cap the pref slice at
    // TI, reducing `other` first — the QDCGT worksheet's min) lands in P3 with the full deduction stack.
    let ordinary_taxable_income = (taxable_income - qual_div - cap_gain_distr).max(Usd::ZERO);

    // ── W-2 SE/Medicare channels (two DIFFERENT aggregations — deep/02 §3.4 / C4) ─────────────────
    // §1402(b)(1) SS cap is PER-INDIVIDUAL: `w2_ss_wages` = the SE-earner's OWN box 3 + box 7 tips, NOT
    // the household sum. The SE earner is the single Schedule C owner (Taxpayer when there is no Sch C).
    let se_owner = ri
        .schedule_c
        .as_ref()
        .map(|c| c.owner)
        .unwrap_or(Owner::Taxpayer);
    let w2_ss_wages: Usd = ri
        .w2s
        .iter()
        .filter(|w| w.owner == se_owner)
        .map(|w| w.box3_ss_wages + w.box7_ss_tips)
        .sum();
    // Form 8959 Part I/II uses HOUSEHOLD-total Medicare wages (both spouses' box 5).
    let w2_medicare_wages: Usd = ri.w2s.iter().map(|w| w.box5_medicare_wages).sum();
    let schedule_c_expenses = ri
        .schedule_c
        .as_ref()
        .map(|c| c.expenses)
        .unwrap_or(Usd::ZERO);

    TaxProfile {
        filing_status: status,
        ordinary_taxable_income,
        magi_excluding_crypto: agi,
        qualified_dividends_and_other_pref_income: qual_div,
        other_net_capital_gain: cap_gain_distr,
        capital_loss_carryforward_in: ri.capital_loss_carryforward_in,
        w2_ss_wages,
        w2_medicare_wages,
        schedule_c_expenses,
    }
}

/// Schedule B §6012 / Form 1040 Schedule B filing threshold ($1,500 for interest and for dividends).
const SCHEDULE_B_THRESHOLD: Usd = dec!(1500);

/// Whether Schedule B must be filed (SPEC §7.1, R3-I2 — the single normative site): **taxable interest >
/// $1,500** OR **ordinary dividends > $1,500** OR `foreign_accounts == Some(true)` (Part III trigger (b)).
/// Uses the NON-crypto 1040 2b / 3b figures (crypto lending interest lands on Sch 1 L8v, not 2b).
/// `foreign_trust == Some(true)` refuses upstream (§4.10) and is never a Schedule-B path.
pub fn schedule_b_files(ri: &ReturnInputs) -> bool {
    sum_taxable_interest(ri) > SCHEDULE_B_THRESHOLD
        || sum_ordinary_dividends(ri) > SCHEDULE_B_THRESHOLD
        || ri.foreign_accounts == Some(true)
}

/// When Schedule B files, Part III lines **7a** (foreign financial accounts) AND **8** (foreign trust)
/// MUST be answered — a `None` tri-state is a fail-loud gap (SPEC §7.1 / I7), never a silent "no".
/// `true` ⇒ Schedule B files but `foreign_accounts` **or** `foreign_trust` is unanswered (the caller
/// refuses rather than guess). (`foreign_trust == Some(true)` refuses earlier as `ForeignTrust`; this
/// catches the unanswered `None`.) Wired into `screen_inputs` (input-screenable, review P2-I1).
pub fn schedule_b_part3_unanswered(ri: &ReturnInputs) -> bool {
    schedule_b_files(ri) && (ri.foreign_accounts.is_none() || ri.foreign_trust.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::IncomeKind;
    use crate::identity::EventId;
    use crate::state::{IncomeRecord, LedgerState};
    use crate::tax::compute::compute_tax_year;
    use crate::tax::return_inputs::{
        Form1099Div, Form1099G, Form1099Int, ScheduleAInputs, ScheduleCInputs, W2,
    };
    use crate::tax::tables::{synthetic_table, TaxTable};
    use crate::tax::types::{Carryforward, TaxOutcome};
    use std::collections::BTreeMap;
    use time::macros::date;

    fn ty2024_params() -> FullReturnParams {
        let mut std_deduction = BTreeMap::new();
        std_deduction.insert(FilingStatus::Single, dec!(14600));
        std_deduction.insert(FilingStatus::Mfj, dec!(29200));
        std_deduction.insert(FilingStatus::Mfs, dec!(14600));
        std_deduction.insert(FilingStatus::HoH, dec!(21900));
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

    fn w2(owner: Owner, box1: Usd, box3: Usd, box5: Usd) -> W2 {
        W2 {
            owner,
            box1_wages: box1,
            box3_ss_wages: box3,
            box5_medicare_wages: box5,
            ..Default::default()
        }
    }

    fn tables_2024() -> BTreeMap<i32, TaxTable> {
        let mut m = BTreeMap::new();
        m.insert(2024, synthetic_table(2024));
        m
    }
    fn income(kind: IncomeKind, business: bool, fmv: Usd) -> IncomeRecord {
        IncomeRecord {
            event: EventId::decision(1),
            recognized_at: date!(2024 - 06 - 01),
            sat: 100_000_000,
            usd_fmv: fmv,
            kind,
            business,
            pseudo: false,
        }
    }
    fn mining(fmv: Usd) -> IncomeRecord {
        income(IncomeKind::Mining, true, fmv)
    }
    fn state_income(recs: Vec<IncomeRecord>) -> LedgerState {
        LedgerState {
            income_recognized: recs,
            ..Default::default()
        }
    }
    fn screened(ri: &ReturnInputs, st: &LedgerState) -> Option<RefuseReason> {
        screen_compute_dependent(ri, st, 2024, &ty2024_params()).map(|r| r.reason)
    }
    /// A Single household the synthetic table can price (it only carries `Single` schedules). Tuned so
    /// the ordinary base (`ordinary_taxable_income`) sits just below the synthetic $100k bracket edge:
    /// wages 98,600 + int 4,000 + ord-div 10,000 + cap-gain-distr 3,000 = AGI 115,600; taxable 101,000;
    /// ordinary base = 101,000 − 8,000 qd − 3,000 cap-gain = 90,000.
    fn single_household() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(98600), dec!(98600), dec!(98600))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(4000),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000),
                box1b_qualified: dec!(8000),
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// The frozen seam, end to end: the derived profile carries NO crypto, so with an empty ledger the
    /// crypto DELTA is exactly zero; adding business-mining ordinary income makes the delta positive —
    /// the engine stacks crypto ON TOP of the derived non-crypto base. (Task-3 exclusion-semantics KAT.)
    #[test]
    fn derived_profile_composes_with_the_frozen_crypto_engine() {
        let p = derive_tax_profile(&single_household(), &ty2024_params(), 2024);
        let tables = tables_2024();

        // No crypto in the ledger ⇒ zero crypto delta (derive injects no phantom crypto).
        match compute_tax_year(&[], &LedgerState::default(), 2024, Some(&p), &tables) {
            TaxOutcome::Computed(r) => assert_eq!(r.total_federal_tax_attributable, Usd::ZERO),
            other => panic!("clean derived profile must compute, got {other:?}"),
        }

        // $60k business mining (ordinary crypto income) ⇒ positive delta, taxed on top of the base.
        let st = LedgerState {
            income_recognized: vec![mining(dec!(60000))],
            ..Default::default()
        };
        match compute_tax_year(&[], &st, 2024, Some(&p), &tables) {
            TaxOutcome::Computed(r) => assert!(r.total_federal_tax_attributable > Usd::ZERO),
            other => panic!("crypto year must compute, got {other:?}"),
        }
    }

    /// A WRONG derivation that forgot to strip the preferential slice (left qd+cap-gain in the ordinary
    /// bottom) changes the crypto tax the engine computes — proving the strip is load-bearing through the
    /// seam, not just a cosmetic profile field. Uses a crypto LTCG so the pref stacking is exercised.
    #[test]
    fn forgetting_to_strip_changes_the_engine_result() {
        let good = derive_tax_profile(&single_household(), &ty2024_params(), 2024);
        // The strip-once bug: ordinary bottom left inflated by the preferential slice.
        let mut bad = good.clone();
        bad.ordinary_taxable_income += good.qualified_dividends_and_other_pref_income
            + good.other_net_capital_gain; // 246,800 → 257,800
        let tables = tables_2024();
        let st = LedgerState {
            income_recognized: vec![mining(dec!(40000))],
            ..Default::default()
        };
        let g = match compute_tax_year(&[], &st, 2024, Some(&good), &tables) {
            TaxOutcome::Computed(r) => r.total_federal_tax_attributable,
            other => panic!("good profile must compute, got {other:?}"),
        };
        let b = match compute_tax_year(&[], &st, 2024, Some(&bad), &tables) {
            TaxOutcome::Computed(r) => r.total_federal_tax_attributable,
            other => panic!("bad profile must compute, got {other:?}"),
        };
        assert_ne!(g, b, "the strip must affect the engine's crypto tax");
    }

    /// deep/02 Worked Example 1 (MFJ, no crypto) — the derived `TaxProfile` cent-exact, every field.
    #[test]
    fn derive_matches_deep02_example1_to_the_cent() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2(Owner::Taxpayer, dec!(180000), dec!(168600), dec!(180000)),
                w2(Owner::Spouse, dec!(90000), dec!(90000), dec!(90000)),
            ],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(4000),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000),
                box1b_qualified: dec!(8000),
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params(), 2024);
        assert_eq!(p.filing_status, FilingStatus::Mfj);
        assert_eq!(p.ordinary_taxable_income, dec!(246800)); // 257,800 − 8,000 − 3,000
        assert_eq!(p.magi_excluding_crypto, dec!(287000)); // AGI
        assert_eq!(p.qualified_dividends_and_other_pref_income, dec!(8000));
        assert_eq!(p.other_net_capital_gain, dec!(3000));
        assert_eq!(p.w2_ss_wages, dec!(168600)); // SE-earner (Taxpayer) OWN box 3, NOT the 258,600 sum
        assert_eq!(p.w2_medicare_wages, dec!(270000)); // household Σ box 5
        assert_eq!(p.schedule_c_expenses, dec!(0));
        assert_eq!(p.capital_loss_carryforward_in, Carryforward::default());
        // Round-trip identity (deep/02 §1.4): taxable_income == ord_ti + qd + cap_gain_distr.
        assert_eq!(
            p.ordinary_taxable_income
                + p.qualified_dividends_and_other_pref_income
                + p.other_net_capital_gain,
            dec!(257800)
        );
    }

    /// "Strip once" — box 1a is used for the ordinary total, box 1b ONLY for the preferential split; a
    /// higher box 1b must NOT lower AGI/ordinary income (the income-side double-count bug, deep/02 §1.4).
    #[test]
    fn box1b_does_not_reduce_agi_or_double_count() {
        // Enough wage income that taxable income clears the standard deduction (so the strip is exercised,
        // not floored to zero).
        let base = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000))],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000),
                box1b_qualified: dec!(2000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut more_qual = base.clone();
        more_qual.div_1099[0].box1b_qualified = dec!(9000); // more of the SAME $10k is qualified
        let a = derive_tax_profile(&base, &ty2024_params(), 2024);
        let b = derive_tax_profile(&more_qual, &ty2024_params(), 2024);
        // AGI unchanged (box 1a is the income; box 1b is only a split) = 100,000 + 10,000.
        assert_eq!(a.magi_excluding_crypto, b.magi_excluding_crypto);
        assert_eq!(a.magi_excluding_crypto, dec!(110000));
        // The larger qualified slice moves MORE out of the ordinary bottom into the preferential channel.
        assert_eq!(b.qualified_dividends_and_other_pref_income, dec!(9000));
        assert!(b.ordinary_taxable_income < a.ordinary_taxable_income);
        // But the difference is exactly the moved slice ($7,000), not a double-count of AGI.
        assert_eq!(a.ordinary_taxable_income - b.ordinary_taxable_income, dec!(7000));
    }

    /// box 2a capital-gain distributions are IN AGI (via L7) AND stripped once — never double-removed.
    #[test]
    fn box2a_is_in_agi_and_stripped_once() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            div_1099: vec![Form1099Div {
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params(), 2024);
        assert_eq!(p.magi_excluding_crypto, dec!(3000)); // in AGI
        assert_eq!(p.other_net_capital_gain, dec!(3000)); // re-enters via preferential channel
        assert_eq!(p.ordinary_taxable_income, Usd::ZERO); // 3,000 − std 14,600 floored, then strip
    }

    /// L1 refund + L7 unemployment raise AGI; L18 early-withdrawal lowers it (Sch 1 non-crypto lines).
    #[test]
    fn schedule_1_noncrypto_income_and_adjustments() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(5000),
                box2_early_withdrawal_penalty: dec!(1000),
                box3_treasury_interest: dec!(2000),
                ..Default::default()
            }],
            g_1099: vec![Form1099G {
                box1_unemployment: dec!(4000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut sch1 = ri.clone();
        sch1.sch1.state_refund_taxable = dec!(600);
        let p = derive_tax_profile(&sch1, &ty2024_params(), 2024);
        // AGI = 100,000 + (5,000+2,000) int + 4,000 unemp + 600 refund − 1,000 early-wd = 110,600.
        assert_eq!(p.magi_excluding_crypto, dec!(110600));
    }

    /// §221 student-loan deduction: full below the range, phased in-range, zero above; MFS ⇒ $0.
    #[test]
    fn student_loan_phaseout_and_mfs_zero() {
        let params = ty2024_params();
        // Single, MAGI below $80k → full $2,500 cap.
        assert_eq!(
            student_loan_deduction(dec!(3000), dec!(60000), FilingStatus::Single, &params),
            dec!(2500)
        );
        // Single, MAGI at the $87,500 midpoint of 80k–95k → half of the capped $2,500 = $1,250.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(87500), FilingStatus::Single, &params),
            dec!(1250)
        );
        // Single, MAGI ≥ $95k → fully phased out.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(95000), FilingStatus::Single, &params),
            Usd::ZERO
        );
        // MFS → always $0 (§221(e)(2)), even below the range.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(40000), FilingStatus::Mfs, &params),
            Usd::ZERO
        );
        // MFJ uses the higher $165k–$195k range: $170k is in-range.
        let d = student_loan_deduction(dec!(2500), dec!(170000), FilingStatus::Mfj, &params);
        assert!(d > Usd::ZERO && d < dec!(2500));
        // QSS is NOT a joint return (§221 — review C2): it uses the $80k–$95k UNMARRIED range like Single,
        // NOT MFJ's $165k+. At $120k MAGI a QSS filer is fully phased out ($0), not granted the full $2,500.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(120000), FilingStatus::Qss, &params),
            Usd::ZERO
        );
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(60000), FilingStatus::Qss, &params),
            dec!(2500)
        );
    }

    /// The derivation flows the student-loan deduction into AGI (Single with $1,000 paid, below range).
    #[test]
    fn derive_applies_student_loan_adjustment() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(50000), dec!(50000), dec!(50000))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(1000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut with_loan = ri.clone();
        with_loan.sch1.student_loan_interest_paid = dec!(1000);
        let p = derive_tax_profile(&with_loan, &ty2024_params(), 2024);
        // AGI = 50,000 + 1,000 − 1,000 student-loan = 50,000.
        assert_eq!(p.magi_excluding_crypto, dec!(50000));
    }

    /// The SE-earner channel: with a spouse-owned Schedule C, `w2_ss_wages` tracks the SPOUSE's box 3,
    /// not the taxpayer's, while Medicare wages stay household-summed.
    #[test]
    fn se_owner_selects_ss_wages_channel() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000)),
                w2(Owner::Spouse, dec!(40000), dec!(40000), dec!(40000)),
            ],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Spouse,
                expenses: dec!(2500),
                ..Default::default()
            }),
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params(), 2024);
        assert_eq!(p.w2_ss_wages, dec!(40000)); // spouse's own box 3
        assert_eq!(p.w2_medicare_wages, dec!(140000)); // household Σ box 5
        assert_eq!(p.schedule_c_expenses, dec!(2500));
    }

    /// Schedule B filing trigger (SPEC §7.1): interest OR dividends > $1,500, or a foreign account.
    #[test]
    fn schedule_b_filing_trigger() {
        let int = |amt: Usd| ReturnInputs {
            filing_status: FilingStatus::Single,
            int_1099: vec![Form1099Int {
                box1_interest: amt,
                ..Default::default()
            }],
            foreign_accounts: Some(false),
            ..Default::default()
        };
        // $2,000 interest → files; exactly $1,500 → does NOT (strictly greater).
        assert!(schedule_b_files(&int(dec!(2000))));
        assert!(!schedule_b_files(&int(dec!(1500))));
        // $2,000 ordinary dividends → files.
        let div = ReturnInputs {
            filing_status: FilingStatus::Single,
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(2000),
                ..Default::default()
            }],
            foreign_accounts: Some(false),
            ..Default::default()
        };
        assert!(schedule_b_files(&div));
        // Below both thresholds but a foreign account is present → files via Part III trigger (b).
        let mut fa = int(dec!(200));
        fa.foreign_accounts = Some(true);
        assert!(schedule_b_files(&fa));
    }

    /// Part III must be answered when Schedule B files — a `None` foreign-accounts tri-state fails loud.
    #[test]
    fn schedule_b_part3_none_is_fail_loud_only_when_filing() {
        // Files ($2,000 interest) but foreign_accounts unanswered → fail-loud.
        let unanswered = ReturnInputs {
            filing_status: FilingStatus::Single,
            int_1099: vec![Form1099Int {
                box1_interest: dec!(2000),
                ..Default::default()
            }],
            foreign_accounts: None,
            ..Default::default()
        };
        assert!(schedule_b_part3_unanswered(&unanswered));
        // Files, foreign_accounts answered but foreign_trust (line 8) unanswered → still fail-loud.
        let trust_unanswered = ReturnInputs {
            filing_status: FilingStatus::Single,
            int_1099: vec![Form1099Int {
                box1_interest: dec!(2000),
                ..Default::default()
            }],
            foreign_accounts: Some(false),
            foreign_trust: None,
            ..Default::default()
        };
        assert!(schedule_b_part3_unanswered(&trust_unanswered));
        // Files with BOTH answered → fine.
        let answered = ReturnInputs {
            foreign_trust: Some(false),
            ..trust_unanswered.clone()
        };
        assert!(!schedule_b_part3_unanswered(&answered));
        // Not filing (small amounts) → a None is fine (Schedule B not required).
        let not_filing = ReturnInputs {
            filing_status: FilingStatus::Single,
            int_1099: vec![Form1099Int {
                box1_interest: dec!(100),
                ..Default::default()
            }],
            foreign_accounts: None,
            ..Default::default()
        };
        assert!(!schedule_b_part3_unanswered(&not_filing));
    }

    // ── §63 standard deduction (Phase 3 task 1) ──────────────────────────────────────────────────
    fn person(dob: Option<Date>, blind: bool) -> Person {
        Person {
            date_of_birth: dob,
            blind,
            ..Default::default()
        }
    }
    fn filer(status: FilingStatus) -> ReturnInputs {
        ReturnInputs {
            filing_status: status,
            ..Default::default()
        }
    }

    /// Basic std deduction + §63(f) aged/blind boxes (unmarried $1,950, married $1,550).
    #[test]
    fn standard_deduction_basic_and_aged_blind() {
        let p = ty2024_params();
        // Single, nothing extra → basic $14,600.
        assert_eq!(standard_deduction(&filer(FilingStatus::Single), &p, 2024, Usd::ZERO), dec!(14600));
        // Single + blind → +$1,950.
        let mut blind = filer(FilingStatus::Single);
        blind.header.taxpayer.blind = true;
        assert_eq!(standard_deduction(&blind, &p, 2024, Usd::ZERO), dec!(16550));
        // MFJ, BOTH spouses 65+ → basic $29,200 + 2 × $1,550 = $32,300.
        let mut mfj = filer(FilingStatus::Mfj);
        mfj.header.taxpayer.date_of_birth = Some(date!(1955 - 06 - 01));
        mfj.header.spouse = Some(person(Some(date!(1955 - 06 - 01)), false));
        assert_eq!(standard_deduction(&mfj, &p, 2024, Usd::ZERO), dec!(32300));
    }

    /// The §63(f) age-65 boundary (born on/before Jan 1 of year−64) and the fail-closed `None` DOB.
    #[test]
    fn aged_boundary_and_none_dob() {
        let p = ty2024_params();
        let mk = |dob| {
            let mut r = filer(FilingStatus::Single);
            r.header.taxpayer.date_of_birth = dob;
            r
        };
        // Born 1960-01-01 → 65 by Jan 1 2025 → aged for TY2024 (14,600 + 1,950).
        assert_eq!(standard_deduction(&mk(Some(date!(1960 - 01 - 01))), &p, 2024, Usd::ZERO), dec!(16550));
        // Born 1960-01-02 → NOT aged.
        assert_eq!(standard_deduction(&mk(Some(date!(1960 - 01 - 02))), &p, 2024, Usd::ZERO), dec!(14600));
        // None DOB → not established → NOT aged (conservative, fail-closed — dob-option-pin).
        assert_eq!(standard_deduction(&mk(None), &p, 2024, Usd::ZERO), dec!(14600));
    }

    /// §63(c)(5) dependent floor: `min(basic, max($1,300, earned + $450))`, with aged/blind still added.
    #[test]
    fn dependent_floor() {
        let p = ty2024_params();
        let mut dep = filer(FilingStatus::Single);
        dep.header.can_be_claimed_as_dependent_taxpayer = true;
        // Earned $0 → max($1,300, $450) = $1,300.
        assert_eq!(standard_deduction(&dep, &p, 2024, Usd::ZERO), dec!(1300));
        // Earned $5,000 → max($1,300, $5,450) = $5,450 (< basic).
        assert_eq!(standard_deduction(&dep, &p, 2024, dec!(5000)), dec!(5450));
        // Earned $20,000 → $20,450 capped at basic $14,600.
        assert_eq!(standard_deduction(&dep, &p, 2024, dec!(20000)), dec!(14600));
        // Dependent + blind → floor base ($1,300) + $1,950 aged/blind.
        let mut db = dep.clone();
        db.header.taxpayer.blind = true;
        assert_eq!(standard_deduction(&db, &p, 2024, Usd::ZERO), dec!(3250));
    }

    /// QSS uses the MFJ basic std ($29,200 via `Qss → Mfj`) AND the married ($1,550) aged/blind rate.
    #[test]
    fn qss_uses_married_basic_and_aged_blind_rate() {
        let p = ty2024_params();
        let mut qss = filer(FilingStatus::Qss);
        qss.header.taxpayer.date_of_birth = Some(date!(1950 - 01 - 01)); // aged
        assert_eq!(standard_deduction(&qss, &p, 2024, Usd::ZERO), dec!(30750)); // 29,200 + 1,550
    }

    // ── Schedule A itemized deduction (Phase 3 task 2) ────────────────────────────────────────────
    /// No Schedule A ⇒ `None` (the filer takes the standard deduction).
    #[test]
    fn schedule_a_none_without_inputs() {
        assert_eq!(
            schedule_a_deduction(&filer(FilingStatus::Single), dec!(100000), Usd::ZERO, &ty2024_params()),
            None
        );
    }

    /// Medical over the 7.5% floor + SALT (income path) capped at $10k + mortgage.
    #[test]
    fn schedule_a_medical_floor_salt_cap_mortgage() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            medical: dec!(10000),                   // − 7.5%·100k = $2,500 allowed
            salt_state_estimated_payments: dec!(5000),
            salt_real_estate: dec!(8000),           // 5d = 5,000 + 8,000 = 13,000 → capped $10,000
            mortgage_interest_1098: dec!(12000),
            ..Default::default()
        });
        // $2,500 + $10,000 + $12,000 + $0 charitable = $24,500.
        assert_eq!(
            schedule_a_deduction(&r, dec!(100000), Usd::ZERO, &ty2024_params()),
            Some(dec!(24500))
        );
    }

    /// §164(b)(5) either/or: election ON ⇒ 5a is the sales-tax amount ONLY (income withholding ignored);
    /// MFS SALT cap is $5,000. Charitable (line 14) adds straight in.
    #[test]
    fn schedule_a_salt_election_and_mfs_cap() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            salt_use_sales_tax: true,
            salt_sales_tax_amount: dec!(3000),
            salt_state_estimated_payments: dec!(9999), // IGNORED under the sales-tax election
            salt_real_estate: dec!(4000),
            ..Default::default()
        });
        // 5d = 3,000 + 4,000 = 7,000 (< cap); + $1,000 charitable = $8,000.
        assert_eq!(
            schedule_a_deduction(&r, dec!(100000), dec!(1000), &ty2024_params()),
            Some(dec!(8000))
        );
        // MFS: $20,000 real-estate tax caps at $5,000.
        let mut mfs = filer(FilingStatus::Mfs);
        mfs.schedule_a = Some(ScheduleAInputs {
            salt_real_estate: dec!(20000),
            ..Default::default()
        });
        assert_eq!(
            schedule_a_deduction(&mfs, dec!(100000), Usd::ZERO, &ty2024_params()),
            Some(dec!(5000))
        );
    }

    /// `derive_tax_profile` takes max(standard, itemized): a big Schedule A beats the standard deduction.
    #[test]
    fn derive_uses_max_of_std_and_itemized() {
        let p = ty2024_params();
        let mut r = filer(FilingStatus::Single);
        r.w2s = vec![w2(Owner::Taxpayer, dec!(200000), dec!(200000), dec!(200000))];
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(30000),
            salt_real_estate: dec!(15000), // capped at $10k
            ..Default::default()
        });
        // Itemized $40,000 > std $14,600 → taxable = $200,000 − $40,000 = $160,000.
        assert_eq!(
            schedule_a_deduction(&r, dec!(200000), Usd::ZERO, &p).unwrap(),
            dec!(40000)
        );
        assert_eq!(derive_tax_profile(&r, &p, 2024).ordinary_taxable_income, dec!(160000));
    }

    /// §63(e) `ForceItemize` uses Schedule A even when it is smaller than the standard deduction.
    #[test]
    fn force_itemize_uses_schedule_a_even_when_smaller() {
        use crate::tax::return_inputs::ItemizeElection;
        let mut r = filer(FilingStatus::Single);
        r.w2s = vec![w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000))];
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(1000),
            ..Default::default()
        });
        r.itemize_election = ItemizeElection::ForceItemize;
        // Forced $1,000 (< std $14,600) → taxable = $100,000 − $1,000 = $99,000.
        assert_eq!(derive_tax_profile(&r, &ty2024_params(), 2024).ordinary_taxable_income, dec!(99000));
    }

    /// §63(c)(6): an MFS filer whose spouse itemizes gets NO standard deduction.
    #[test]
    fn mfs_spouse_itemizes_forces_zero_std() {
        let p = ty2024_params();
        let mut r = filer(FilingStatus::Mfs);
        r.w2s = vec![w2(Owner::Taxpayer, dec!(50000), dec!(50000), dec!(50000))];
        r.mfs_spouse_itemizes = Some(true); // spouse itemizes → std = 0, no Sch A → taxable = $50,000.
        assert_eq!(derive_tax_profile(&r, &p, 2024).ordinary_taxable_income, dec!(50000));
        // Spouse does NOT itemize → MFS std $14,600 → taxable = $35,400.
        r.mfs_spouse_itemizes = Some(false);
        assert_eq!(derive_tax_profile(&r, &p, 2024).ordinary_taxable_income, dec!(35400));
    }

    // ── Compute-dependent refuse rows (task 2) ───────────────────────────────────────────────────
    fn single() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        }
    }

    /// Business-flagged crypto Interest has no clean v1 home → refuse (R3-I3).
    #[test]
    fn business_interest_income_refuses() {
        let st = state_income(vec![income(IncomeKind::Interest, true, dec!(5000))]);
        assert_eq!(
            screened(&single(), &st),
            Some(RefuseReason::BusinessInterestIncome)
        );
        // The SAME interest as NON-business (hobby lending) does NOT refuse — it lands on Sch 1 L8v.
        let hobby = state_income(vec![income(IncomeKind::Interest, false, dec!(5000))]);
        assert_eq!(screened(&single(), &hobby), None);
    }

    /// SE-eligible business crypto income with no Schedule C ⇒ fail loud (owner/description unknowable).
    #[test]
    fn business_income_without_schedule_c_fails_loud() {
        let st = state_income(vec![mining(dec!(50000))]);
        assert_eq!(
            screened(&single(), &st),
            Some(RefuseReason::BusinessIncomeWithoutScheduleC)
        );
    }

    /// Schedule C net < 0 (expenses exceed business gross) ⇒ refuse; a net profit does not.
    #[test]
    fn schedule_c_loss_refuses_but_profit_does_not() {
        let with_sc = |expenses: Usd| ReturnInputs {
            schedule_c: Some(ScheduleCInputs {
                expenses,
                ..Default::default()
            }),
            ..single()
        };
        let st = state_income(vec![mining(dec!(50000))]);
        // $50k gross − $60k expenses = −$10k loss → refuse.
        assert_eq!(
            screened(&with_sc(dec!(60000)), &st),
            Some(RefuseReason::ScheduleCLoss)
        );
        // $50k gross − $10k expenses = $40k profit → OK.
        assert_eq!(screened(&with_sc(dec!(10000)), &st), None);
    }

    /// §1(g) kiddie tax: a claimable-as-dependent filer with unearned income (interest + hobby crypto)
    /// over the $2,600 threshold ⇒ refuse; below threshold, or non-dependent, ⇒ no refusal.
    #[test]
    fn kiddie_tax_refuses_dependent_over_threshold() {
        let dependent = |interest: Usd| {
            let mut ri = single();
            ri.header.can_be_claimed_as_dependent_taxpayer = true;
            ri.int_1099 = vec![Form1099Int {
                box1_interest: interest,
                ..Default::default()
            }];
            ri
        };
        let empty = LedgerState::default();
        // $3,000 interest > $2,600 → refuse.
        assert_eq!(screened(&dependent(dec!(3000)), &empty), Some(RefuseReason::KiddieTax));
        // $2,000 interest ≤ $2,600 → no refusal.
        assert_eq!(screened(&dependent(dec!(2000)), &empty), None);
        // Non-business (hobby) crypto reward counts as unearned too: $2,000 interest + $1,000 reward > $2,600.
        let hobby = state_income(vec![income(IncomeKind::Reward, false, dec!(1000))]);
        assert_eq!(screened(&dependent(dec!(2000)), &hobby), Some(RefuseReason::KiddieTax));
        // NOT claimable as a dependent ⇒ never kiddie, even with high unearned income.
        let mut not_dep = dependent(dec!(9000));
        not_dep.header.can_be_claimed_as_dependent_taxpayer = false;
        assert_eq!(screened(&not_dep, &empty), None);
    }

    /// Wages (earned) do NOT count toward the kiddie unearned threshold — a working dependent with big
    /// wages but small investment income is not kiddie-refused.
    #[test]
    fn kiddie_excludes_earned_wages() {
        let mut ri = single();
        ri.header.can_be_claimed_as_dependent_taxpayer = true;
        ri.w2s = vec![w2(Owner::Taxpayer, dec!(20000), dec!(20000), dec!(20000))]; // earned
        ri.int_1099 = vec![Form1099Int {
            box1_interest: dec!(500),
            ..Default::default()
        }]; // unearned $500 < $2,600
        assert_eq!(screened(&ri, &LedgerState::default()), None);
    }
}
