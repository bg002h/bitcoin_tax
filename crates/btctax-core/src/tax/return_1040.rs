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
use crate::state::{LedgerState, RemovalKind, Term};
use crate::tax::amt::amt_should_file_6251;
use crate::tax::charitable::apply_170b;
use crate::tax::compute::{net_1222, CapNet};
use crate::tax::method::qdcgt_line16;
use crate::tax::other_taxes::{form_8959, form_8960, sch2_line4_se, Form8959, Form8960};
use crate::tax::qbi::{compute_8995, qbi_over_threshold};
use crate::tax::return_inputs::{
    CarryProvenance, CharitableCarryItem, CharitableClass, CharitableGift, Owner, ReturnInputs,
};
use crate::tax::return_refuse::{Refusal, RefuseReason};
use crate::tax::se::{compute_se_tax, se_net_income, SeTaxResult};
use crate::tax::tables::{loss_limit, FullReturnParams, TaxTable, EMPLOYEE_OASDI_RATE};
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
pub(crate) fn is_aged(dob: Option<Date>, year: i32) -> bool {
    let Some(d) = dob else {
        return false;
    };
    Date::from_calendar_date(year - 64, Month::January, 1).is_ok_and(|cutoff| d <= cutoff)
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

    // ★ The SAME box count the 1040's §63(f) checkboxes print (`AgedBlindBoxes::for_return`). The IRS
    // validates a nonstandard standard deduction by COUNTING those boxes, so the count and the amount
    // must come from one derivation — two would let a filed return claim an addition its own checkboxes
    // do not support (`p6-aged-blind-checkboxes-missing`).
    let boxes = crate::tax::packet::AgedBlindBoxes::for_return(ri, year).count();
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

/// The §213(a) medical-expense floor: 7.5% of AGI (Schedule A line 3).
pub const MEDICAL_FLOOR_RATE: Usd = dec!(0.075);

/// The **Schedule D components** — the §1222 netting, by the form's own lines.
///
/// The frozen `net_1222` engine IS Schedule D: its `st_net` is line 7 (crypto short-term plus the
/// line-6 carryover), its `lt_net` is line 15 (crypto long-term plus the line-13 capital-gain
/// distributions and the line-14 carryover), and its `loss_deduction` is the §1211(b) amount on
/// line 21. Nothing here re-derives any of it.
///
/// **Lines 6, 14 and 21 are PARENTHESIZED boxes on the printed form** — the form supplies the minus
/// sign — so all three are stored as POSITIVE MAGNITUDES. `st_carryover_6` and `lt_carryover_14` are
/// the prior-year carryforward magnitudes as entered; `loss_deduction_21` is the allowed §1211(b)
/// offset (≤ $3,000 / $1,500 MFS), also a magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleDParts {
    /// L3 (d) — short-term proceeds from Form 8949 (Box C or **Box I**, the digital-asset box).
    pub st_proceeds_3d: Usd,
    /// L3 (e) — short-term cost basis.
    pub st_cost_3e: Usd,
    /// L3 (h) — short-term gain or loss (signed).
    pub st_gain_3h: Usd,
    /// L6 — prior-year SHORT-term capital loss carryover. **Positive magnitude** (paren box).
    pub st_carryover_6: Usd,
    /// L7 — net short-term gain or loss (signed) = `CapNet::st_net`.
    pub st_net_7: Usd,
    /// L10 (d) — long-term proceeds from Form 8949 (Box F or **Box L**).
    pub lt_proceeds_10d: Usd,
    /// L10 (e) — long-term cost basis.
    pub lt_cost_10e: Usd,
    /// L10 (h) — long-term gain or loss (signed).
    pub lt_gain_10h: Usd,
    /// L13 — capital gain distributions (Σ 1099-DIV box 2a; long-term in character).
    pub cap_gain_distr_13: Usd,
    /// L14 — prior-year LONG-term capital loss carryover. **Positive magnitude** (paren box).
    pub lt_carryover_14: Usd,
    /// L15 — net long-term gain or loss (signed) = `CapNet::lt_net`.
    pub lt_net_15: Usd,
    /// L16 — combine 7 and 15 (signed).
    pub total_16: Usd,
    /// L21 — the §1211(b) allowed loss offset. **Positive magnitude** (paren box); zero unless L16 < 0.
    pub loss_deduction_21: Usd,
    /// 1040 line 3a — qualified dividends. Not a Schedule D line, but line 22 asks whether there are
    /// any, and the answer routes the tax computation (QDCGT vs the Tax Table).
    pub qualified_dividends: Usd,
}

/// The **Schedule C components** — the crypto trade-or-business profit-and-loss lines.
///
/// v1 models exactly ONE Schedule C (the crypto trade or business). The filer supplies only a flat
/// expense total; v1 does not itemize Part II, so Schedule C's expense lines 8–27a are BLANK and only
/// the **line 28 total** is printed. There is no cost of goods sold (Part III), no vehicle info
/// (Part IV), and no home-office deduction (line 30) — mining/staking has no inventory, and the §280A
/// home-office computation is out of scope.
///
/// **A Schedule C LOSS is REFUSED upstream** (§465 at-risk substantiation is out of scope), so the
/// net profit is always ≥ 0 and line 31 never needs the loss checkboxes (32a/32b).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleCParts {
    /// L1 — gross receipts: Σ business SE-eligible crypto income (mining/staking/rewards flagged as a
    /// trade or business). Business-flagged crypto `Interest` is NOT here — it refuses (R3-I3).
    pub gross_receipts_1: Usd,
    /// L28 — total expenses (the filer's flat total; Part II's individual lines are not itemized).
    pub total_expenses_28: Usd,
    /// L31 — net profit = line 7 − line 28, floored at 0 (a loss refuses upstream). Flows to BOTH
    /// Schedule 1 line 3 AND Schedule SE line 2 — the same figure, two destinations.
    pub net_profit_31: Usd,
}

/// The **Schedule 1 components** — the income and adjustment lines that feed 1040 L8 and L10.
///
/// Exact cents (this is the computation; `printed::schedule_1_lines` rounds at the line and re-adds
/// the ROUNDED lines so the filed form cross-foots — SPEC §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule1Parts {
    /// L1 — taxable refunds/credits/offsets of state and local income taxes (§111 tax-benefit rule).
    pub state_refund_1: Usd,
    /// L3 — business income: the crypto **Schedule C** net (gross SE income − expenses, floored at 0;
    /// a Schedule C LOSS is refused upstream, §465 at-risk being out of scope).
    pub schedule_c_net_3: Usd,
    /// L7 — unemployment compensation.
    pub unemployment_7: Usd,
    /// L8v — "Digital assets received as ordinary income not reported elsewhere": the NON-business
    /// crypto ordinary income (mining/staking/rewards that are not a trade or business).
    pub crypto_ordinary_8v: Usd,
    /// L15 — the §164(f) deductible part of self-employment tax (one-half of SS + regular Medicare;
    /// the §1401(b)(2) Additional Medicare Tax is expressly excluded).
    pub half_se_15: Usd,
    /// L18 — penalty on early withdrawal of savings (Σ 1099-INT box 2).
    pub early_withdrawal_18: Usd,
    /// L21 — the §221 student-loan interest deduction, after its MAGI phase-out.
    pub student_loan_21: Usd,
}

/// The **Schedule A components**, line by line — the itemized deduction is the SUM of these, and the
/// P6 printed chain needs the individual lines, not just the total.
///
/// Exact cents throughout (this is the computation, not the printed form; `printed::schedule_a_lines`
/// rounds each line half-up and re-adds the ROUNDED lines so the filed form cross-foots — SPEC §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleAParts {
    /// L1 — medical and dental expenses (as entered).
    pub medical_expenses: Usd,
    /// L2 — AGI (the floor's base). **Clamped at 0**: a negative AGI would shrink the 7.5% floor below
    /// zero and INFLATE the medical deduction, so the floor can never help the taxpayer.
    pub agi: Usd,
    /// L3 — the §213(a) floor = 7.5% × line 2.
    pub medical_floor: Usd,
    /// L4 — medical allowed = max(0, line 1 − line 3).
    pub medical_allowed: Usd,
    /// L5a — state/local income taxes, OR (on the §164(b)(5) election) general sales taxes.
    pub salt_5a: Usd,
    /// L5b — state/local real-estate taxes.
    pub salt_5b: Usd,
    /// L5c — state/local personal-property taxes.
    pub salt_5c: Usd,
    /// L5d — add 5a through 5c.
    pub salt_5d: Usd,
    /// L5e — the §164(b) SALT cap applied: min(line 5d, [`ScheduleAParts::salt_cap`]).
    pub salt_5e: Usd,
    /// The §164(b) cap itself ($10,000; $5,000 MFS). Carried because the PRINTED line 5e must cap the
    /// PRINTED line 5d — the printed chain cannot re-derive the cap without doing tax logic in the
    /// forms crate, which is precisely what it must not do.
    pub salt_cap: Usd,
    /// L8a — home-mortgage interest reported on Form 1098.
    pub mortgage_8a: Usd,
    /// L11 — current-year CASH charitable contributions allowed (§170(b)-limited).
    pub charitable_cash_11: Usd,
    /// L12 — current-year NONCASH contributions allowed, including crypto donations.
    pub charitable_noncash_12: Usd,
    /// L13 — prior-year charitable CARRYOVER allowed this year.
    pub charitable_carryover_13: Usd,
    /// L14 — total charitable allowed = 11 + 12 + 13.
    pub charitable_14: Usd,
    /// L17 — total itemized deductions = 4 + 5e (+ 6, 7) + 8a (+ 9, 10) + 14 (+ 15, 16) → 1040 L12.
    pub total_17: Usd,
}

/// The **Schedule A components** at `agi`, given the already-§170(b)-limited charitable result (its
/// `allowed_cash`/`allowed_noncash`/`allowed_carryover` ARE Schedule A lines 11/12/13). `None` when the
/// filer has no Schedule A (takes the standard deduction).
///
/// `agi` is the caller's AGI: the derivation passes NON-crypto AGI (and non-crypto charitable); the
/// absolute return passes with-crypto AGI (+ crypto donations) — a documented delta-vs-absolute divergence
/// (§6) whenever an AGI-sensitive line (medical floor, charitable ceiling) binds.
pub fn schedule_a_parts(
    ri: &ReturnInputs,
    agi: Usd,
    charitable: &crate::tax::charitable::CharitableResult,
    params: &FullReturnParams,
) -> Option<ScheduleAParts> {
    let a = ri.schedule_a.as_ref()?;
    // A negative AGI would shrink the 7.5% floor below zero and inflate the medical deduction; clamp it so the
    // floor never helps the taxpayer (review M1). Mirrors the same clamp inside `apply_170b`.
    let agi = agi.max(Usd::ZERO);

    // Lines 1-4 — medical/dental over the §213(a) 7.5%-of-AGI floor.
    let medical_floor = MEDICAL_FLOOR_RATE * agi;
    let medical_allowed = (a.medical - medical_floor).max(Usd::ZERO);

    // Lines 5a-5e — SALT, §164(b)(5) either/or, capped at $10,000 ($5,000 MFS).
    let salt_5a = salt_line_5a(ri, a);
    let salt_5d = salt_5a + a.salt_real_estate + a.salt_personal_property;
    let salt_cap = if ri.filing_status == FilingStatus::Mfs {
        params.salt_cap / dec!(2)
    } else {
        params.salt_cap
    };
    let salt_5e = salt_5d.min(salt_cap);

    // Line 8a — home-mortgage interest (points/8b are refuse-or-advise).
    let mortgage_8a = a.mortgage_interest_1098;

    Some(ScheduleAParts {
        medical_expenses: a.medical,
        agi,
        medical_floor,
        medical_allowed,
        salt_5a,
        salt_5b: a.salt_real_estate,
        salt_5c: a.salt_personal_property,
        salt_5d,
        salt_5e,
        salt_cap,
        mortgage_8a,
        charitable_cash_11: charitable.allowed_cash,
        charitable_noncash_12: charitable.allowed_noncash,
        charitable_carryover_13: charitable.allowed_carryover,
        charitable_14: charitable.allowed,
        total_17: medical_allowed + salt_5e + mortgage_8a + charitable.allowed,
    })
}

/// The **Schedule A itemized deduction total** (line 17) — the sum of [`schedule_a_parts`].
///
/// Kept as a thin wrapper so there is exactly ONE derivation of the itemized deduction: a second one
/// would be free to drift from the printed form's lines, which is the whole failure mode this phase
/// exists to prevent.
pub fn schedule_a_deduction(
    ri: &ReturnInputs,
    agi: Usd,
    charitable: &crate::tax::charitable::CharitableResult,
    params: &FullReturnParams,
) -> Option<Usd> {
    schedule_a_parts(ri, agi, charitable, params).map(|p| p.total_17)
}

/// §63(e)/(c)(6) deduction CHOICE: `max(standard, itemized)` by default; `ForceItemize` honors §63(e)
/// (itemize even if smaller); **MFS with an itemizing spouse** forces this filer's standard deduction to
/// $0 (§63(c)(6) — the spouses must agree). `itemized` is `None` when there is no Schedule A.
fn choose_deduction(ri: &ReturnInputs, standard: Usd, itemized: Option<Usd>) -> Usd {
    use crate::tax::return_inputs::ItemizeElection;
    let itemized = itemized.unwrap_or(Usd::ZERO);
    // §63(c)(6): an MFS filer whose spouse itemizes gets NO standard deduction (a `None` tri-state on MFS
    // is refused upstream — G15).
    let standard = if ri.filing_status == FilingStatus::Mfs && ri.mfs_spouse_itemizes == Some(true)
    {
        Usd::ZERO
    } else {
        standard
    };
    match ri.itemize_election {
        ItemizeElection::ForceItemize => itemized,
        ItemizeElection::Auto => standard.max(itemized),
    }
}

/// Whether [`choose_deduction`] took the ITEMIZED deduction (for the dual-report label — Fable r1 M1/r2
/// Nit). Mirrors the election exactly: `ForceItemize` ⇒ itemized always (§63(e), even with a `None`
/// Schedule A that makes it $0 — `choose_deduction` still returns the itemized arm there); `Auto` ⇒
/// itemized iff it exceeds the (MFS-§63(c)(6)-coerced) standard (equality → standard, matching `.max`).
fn itemized_was_chosen(ri: &ReturnInputs, standard: Usd, itemized: Option<Usd>) -> bool {
    use crate::tax::return_inputs::ItemizeElection;
    if ri.itemize_election == ItemizeElection::ForceItemize {
        return true;
    }
    let Some(itemized) = itemized else {
        return false;
    };
    let standard = if ri.filing_status == FilingStatus::Mfs && ri.mfs_spouse_itemizes == Some(true)
    {
        Usd::ZERO
    } else {
        standard
    };
    itemized > standard
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
    /// Σ non-business crypto **lending interest** (kind == Interest) — the §1411(c)(1)(A)(i) investment
    /// interest subset of `nonbusiness_ordinary` that enters Form 8960 NII (as a line-7 modification, R3-M5;
    /// it rides Sch 1 L8v, NOT 1040 2b). Hobby mining/staking/airdrop/reward stays OUT of NII.
    nonbusiness_lending_interest: Usd,
}

fn crypto_income(state: &LedgerState, year: i32) -> CryptoIncome {
    let mut business_interest = Usd::ZERO;
    let mut nonbusiness_ordinary = Usd::ZERO;
    let mut nonbusiness_lending_interest = Usd::ZERO;
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
            if i.kind == IncomeKind::Interest {
                nonbusiness_lending_interest += i.usd_fmv;
            }
        }
    }
    CryptoIncome {
        business_se_gross: se_net_income(state, year), // canonical business SE-eligible sum
        business_interest,
        nonbusiness_ordinary,
        nonbusiness_lending_interest,
    }
}

/// The §1222/§1211 capital netting for `year`: crypto Schedule D ST/LT nets + box-2a capital-gain
/// distributions (LT character), with the §1212 carryforward-in applied. The single source for 1040 L7
/// ([`capital_gain_line7`]), the QDCGT net-LTCG (`preferential_gain`, → L16), and the Form 8995
/// net-capital-gain (`preferential_gain`, → line 12).
fn capital_net(ri: &ReturnInputs, state: &LedgerState, year: i32, status: FilingStatus) -> CapNet {
    let sd = schedule_d(state, year); // raw crypto ST/LT nets (traverses state.disposals)
    let cf = ri.capital_loss_carryforward_in;
    net_1222(
        sd.st.gain,
        sd.lt.gain,
        sum_cap_gain_distr(ri), // box 2a is LT-character "other" capital gain
        cf.short,
        cf.long,
        loss_limit(status),
    )
}

/// The amount reaching **1040 L7** (capital gain or loss) for `year`: crypto Schedule D nets + box-2a
/// capital-gain distributions, run through §1222 within-character netting + the §1211 loss limit. In a
/// gain year this is the full net gain; in a loss year it is the −$3,000/−$1,500-MFS limited loss.
fn capital_gain_line7(
    ri: &ReturnInputs,
    state: &LedgerState,
    year: i32,
    status: FilingStatus,
) -> Usd {
    let net = capital_net(ri, state, year, status);
    net.ordinary_gain + net.preferential_gain - net.loss_deduction
}

/// The WITH-crypto Schedule A charitable gifts from the ledger's §170(e)-reduced **donations** for
/// `year` (SPEC §4.6; the `p3-crypto-donation-delta-integration` P4 requirement — the absolute Schedule
/// A includes crypto donations, unlike the derive-side non-crypto profile). Per §170(e): a **long-term**
/// donation leg deducts FMV → `CapGainProp30`; a **short-term** leg deducts §170(e) basis `min(FMV,
/// basis)` → `OrdinaryProp50`. Both are 50%-org classes, so `apply_170b`'s "50%-org only" precondition
/// holds by construction. The per-leg sums reconcile with `Removal.claimed_deduction`
/// (`Σ(LT→fmv; ST→min(fmv,basis))`) — this partitions that total by holding-period class.
fn crypto_charitable_gifts(state: &LedgerState, year: i32) -> Vec<CharitableGift> {
    let mut long_fmv = Usd::ZERO; // LT capital-gain property → CapGainProp30 (FMV)
    let mut short_basis = Usd::ZERO; // ST §170(e) ordinary/basis property → OrdinaryProp50
    for r in state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
    {
        for leg in &r.legs {
            match leg.term {
                Term::LongTerm => long_fmv += leg.fmv_at_transfer,
                Term::ShortTerm => short_basis += leg.fmv_at_transfer.min(leg.basis),
            }
        }
    }
    let mut gifts = Vec::new();
    if long_fmv > Usd::ZERO {
        gifts.push(CharitableGift {
            class: CharitableClass::CapGainProp30,
            amount: long_fmv,
        });
    }
    if short_basis > Usd::ZERO {
        gifts.push(CharitableGift {
            class: CharitableClass::OrdinaryProp50,
            amount: short_basis,
        });
    }
    gifts
}

/// §6413(c) **excess Social Security** credit (Schedule 3 line 11): PER PERSON `max(0, Σ box4 − MAX)`,
/// MAX = 6.2% × the year's SS wage base, summed over taxpayer + spouse — **never pooled** (§4.9). Each
/// employer's box4 ≤ MAX (single-employer over-withholding refuses upstream via `SingleEmployerExcessSs`),
/// and a single-employer person nets 0, so the "requires ≥ 2 employers" rule falls out naturally.
fn excess_social_security(ri: &ReturnInputs, table: &TaxTable) -> Usd {
    let max = table.ss_wage_base * EMPLOYEE_OASDI_RATE;
    let per_person = |owner: Owner| -> Usd {
        let withheld: Usd = ri
            .w2s
            .iter()
            .filter(|w| w.owner == owner)
            .map(|w| w.box4_ss_withheld)
            .sum();
        (withheld - max).max(Usd::ZERO)
    };
    per_person(Owner::Taxpayer) + per_person(Owner::Spouse)
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
    let charitable = crate::tax::charitable::apply_170b(
        agi,
        ri.schedule_a.as_ref().map_or(&[][..], |a| &a.charitable),
        &ri.charitable_carryover_in,
        year,
    );
    let itemized = schedule_a_deduction(ri, agi, &charitable, params);
    let deduction = choose_deduction(ri, full_std, itemized);
    let taxable_income = (agi - deduction).max(Usd::ZERO); // 1040 L15 (non-crypto)
                                                           // Strip the preferential slice (qualified div + LT cap-gain distr) EXACTLY ONCE — the engine re-adds
                                                           // it on top of the ordinary bottom via `other_net_capital_gain` + the QD channel (deep/02 §1.4).
                                                           // KNOWN APPROXIMATION (audit-M2 / review M1, → `p2-pref-over-ti-clamp` FOLLOWUP): when
                                                           // `TI < qd + cap_gain_distr` (low ordinary income + large preferential income — e.g. a retiree), the
                                                           // `.max(0)` floors the ordinary base to 0 while the FULL pref slice still reaches the frozen engine
                                                           // (which stacks `qd + pref_gain` with no min-against-TI cap). The reconstructed TI is then ≥ the true
                                                           // TI, so the delta/planning number can only OVERSTATE, never understate (conservative). Exact handling
                                                           // (cap the pref slice at TI, reducing `other` first — the QDCGT worksheet's min) RE-SCHEDULED to P4
                                                           // (review I2): the cap reduces the pref income that feeds the frozen engine, which is the same channel
                                                           // P4's absolute assembly and crypto-delta stacking rewire — doing it here would be undone there. The
                                                           // larger P3 Schedule A deductions make this region more reachable but never flip the conservative sign.
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

// ── §6017 self-employment-tax filing floor: no SE tax (and no ½-SE deduction, no Schedule SE) unless net
//    earnings from self-employment — the 92.35%-factored `base` — are $400 or more (R3-M3 / SPEC §5 stage 7).
const SE_6017_FLOOR: Usd = dec!(400);

/// The **absolute** (WITH-crypto) 1040 assembly — the filed-return counterpart to [`derive_tax_profile`]'s
/// frozen non-crypto `TaxProfile`. Built incrementally across Phase 4; **this increment covers SPEC §5
/// stages 1–9** — income L1a–L9, adjustments L10, AGI L11, deductions L12–L15, regular tax L16, the
/// other-taxes forms (Sch 2 L4 SE, Form 8959, absolute Form 8960), the §904(j) FTC + conservative-omission
/// CTC (L19 = 0), **1040 total tax L24**, and **payments → refund/owed** (§6413(c) excess-SS, withholding
/// L25, total payments L33, refund L35a / owed L37). The remaining P4 increment is the §6 dual report. The
/// §4.10 compute-dependent refuses that need L12/L15/L16 (QBI-above-threshold, AMT screen, TI≤0-with-
/// carryforward) are screened by [`screen_absolute`] after this (infallible) assembly.
///
/// Unlike the derivation, this reads the crypto ledger `state` directly (`capital_gain_line7`,
/// `crypto_income`, `compute_se_tax`) and produces the with-crypto AGI (L11) — the §6 / Form 8960-MAGI /
/// phase-out pivot. It assumes both refuse screens (`screen_inputs` + `screen_compute_dependent`) have
/// already passed, so Schedule C net is non-negative and business-Interest / no-Schedule-C are excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteReturn {
    /// 1040 L1a — Σ W-2 box 1 wages.
    pub wages: Usd,
    /// 1040 L2b — taxable interest (Σ 1099-INT box 1 + box 3).
    pub taxable_interest: Usd,
    /// 1040 L3b — ordinary dividends (Σ 1099-DIV box 1a; INCLUDES the qualified subset).
    pub ordinary_dividends: Usd,
    /// 1040 L3a — qualified dividends (Σ 1099-DIV box 1b) — a preferential subset of L3b, kept for L16.
    pub qualified_dividends: Usd,
    /// 1040 L7 — capital gain/loss: crypto Schedule D nets + box-2a distributions, §1222-netted and
    /// §1211-loss-limited (`capital_gain_line7`).
    pub capital_gain: Usd,
    /// 1040 L8 = Schedule 1 L10 — state refund + unemployment + Schedule C net (crypto business) + L8v
    /// non-business crypto ordinary income.
    pub schedule_1_income: Usd,
    /// 1040 L9 — total income = L1a + L2b + L3b + L7 + L8.
    pub total_income: Usd,
    /// 1040 L10 = Schedule 1 L26 — adjustments: ½-SE (L15) + early-withdrawal penalty (L18) + student-loan
    /// interest (L21).
    pub adjustments: Usd,
    /// Schedule 1 L15 — the §164(f) one-half-SE-tax deduction (0 when the §6017 floor is not met); a
    /// component of `adjustments`.
    pub half_se_deduction: Usd,
    /// 1040 L11 — **with-crypto AGI** (the §6 dual-report / Form 8960-MAGI / phase-out pivot, G7).
    pub agi: Usd,
    /// The §6017-floored Schedule SE result (`None` when there is no SE-eligible business income, it is
    /// fully expensed, or the `base` is below the $400 floor). Consumed by later stages (Sch 2 L4 =
    /// `ss + medicare`; Form 8959 Part II reads `addl`).
    pub se: Option<SeTaxResult>,
    /// The §63 **standard deduction** alternative — basic + §63(f) aged/blind + §63(c)(5) dependent floor
    /// (with the G21 with-crypto earned income = wages + Schedule C net − ½-SE). One arm of L12's `max`.
    pub standard_deduction: Usd,
    /// Schedule A **line 17** itemized total (medical over the 7.5% floor + SALT + mortgage + §170(b)-
    /// limited charitable INCLUDING the ledger's crypto donations, at with-crypto AGI G7). `None` when
    /// the filer has no Schedule A. The other arm of L12's `max`.
    pub itemized_deduction: Option<Usd>,
    /// The Schedule 1 **components** (the income + adjustment lines behind 1040 L8 and L10).
    pub schedule_1: Schedule1Parts,
    /// The Schedule C **components**, when there is a crypto trade or business — `None` otherwise.
    pub schedule_c: Option<ScheduleCParts>,
    /// The Schedule D **components** — the §1222 netting by the form's own lines.
    pub schedule_d: ScheduleDParts,
    /// The Schedule A **components** (lines 1–17), when the filer has a Schedule A — `None` otherwise.
    /// Present even when the STANDARD deduction wins: Schedule A is still computed (that is how the
    /// max() is taken), and the printed return needs the lines to know it was not the better choice.
    /// The P6 printed chain (`printed::schedule_a_lines`) rounds these at the line and re-adds the
    /// ROUNDED lines, so the filed form cross-foots (SPEC §3.1).
    pub schedule_a: Option<ScheduleAParts>,
    /// 1040 **L12** — the deduction taken = `choose_deduction(standard, itemized)` (max, or `ForceItemize`
    /// / MFS-coupled §63(c)(6)).
    pub deduction: Usd,
    /// Whether L12 is the ITEMIZED deduction (vs the standard) — the actual §63(e)/§63(c)(6) election,
    /// for the dual-report label (not re-derivable from the amounts under `ForceItemize`/MFS coupling).
    pub deduction_is_itemized: bool,
    /// 1040 **L13** — the Form 8995 QBI deduction (REIT §199A dividends; 0 when there is no QBI).
    pub qbi_deduction: Usd,
    /// 1040 **L14** = L12 + L13 (deduction + QBI).
    pub total_deductions: Usd,
    /// 1040 **L15** — taxable income = `max(0, AGI − L14)` (with-crypto).
    pub taxable_income: Usd,
    /// The §1(h) preferential net capital gain (QDCGT net-LTCG / Form 8995 net-capital-gain), ≥ 0 — the
    /// preferential slice of L7 (`CapNet::preferential_gain`), kept for L16 and the QBI income limit.
    pub net_ltcg: Usd,
    /// §170(d)(1) charitable carryover to next year (per class / vintage) from the WITH-crypto Schedule A —
    /// the REAL filed carryover (ages even in a standard-deduction year, G8). For the P4 write-back.
    pub charitable_carryover_out: Vec<CharitableCarryItem>,
    /// Form 8995 **line 17** — the REIT/PTP loss carryforward to next year (magnitude). For the write-back.
    pub qbi_reit_ptp_carryforward_out: Usd,
    /// 1040 **L16** — the regular tax on taxable income (whole dollars): the Qualified Dividends & Capital
    /// Gain Tax Worksheet ([`qdcgt_line16`]) on the WITH-crypto L15 / L3a / preferential net LTCG. It
    /// reduces to the plain Tax Table / TCW when there is no preferential income, so it is correct across
    /// all four Schedule-D routing paths (SPEC §7.2). The QDCGT `min(L1, qd+ltcg)` cap (the
    /// `p2-pref-over-ti-clamp` fix) is built into the worksheet, so the absolute L16 never overstates.
    pub regular_tax: Usd,
    /// Schedule 2 **line 4** — self-employment tax = §1401(a) SS + §1401(b)(1) Medicare (the §1401(b)(2)
    /// 0.9% is unbundled to `additional_medicare` Part II, deep/02 C5). 0 when there is no SE tax.
    pub se_tax_sch2_l4: Usd,
    /// Form 8959 — Additional Medicare Tax: Part I (wages) + Part II (SE `addl`) → Sch 2 L11; Part V
    /// withholding → 1040 25c.
    pub additional_medicare: Form8959,
    /// Form 8960 — the ABSOLUTE Net Investment Income Tax (→ Sch 2 L12), NII rebuilt from line items
    /// (full 3b dividends + 2b interest + §1211-limited L7 + crypto lending interest; MAGI = AGI). NOT the
    /// frozen delta engine's `nii_with` — the §6 divergence.
    pub niit: Form8960,
    /// §904(j) foreign-tax credit → Schedule 3 **line 1** = Σ (1099-INT box 6 + 1099-DIV box 7). The
    /// ≤ $300/$600-passive-1099 eligibility is enforced by `screen_inputs` (over the ceiling refuses), so
    /// this is the full amount. Nonrefundable — capped by the tax at L22.
    pub foreign_tax_credit: Usd,
    /// 1040 **L19** — CTC/ODC, a **conservative omission** (§3.4): always 0 in v1, with a loud advisory
    /// (surfaced at render, P5). Never understates (omitting a favorable credit only overstates tax).
    pub ctc_odc_credit: Usd,
    /// 1040 **L22** — income tax after nonrefundable credits = `max(0, L18 − L21)` where L18 = L16 + Sch 2
    /// L17 (AMT/APTC = 0 for a computed return) and L21 = L19 + L20 (nonrefundable credits, v1: FTC).
    pub tax_after_credits: Usd,
    /// Schedule 2 **line 21** → 1040 **L23** — total other taxes = SE (L4) + Additional Medicare (L11) +
    /// NIIT (L12).
    pub schedule_2_other_taxes: Usd,
    /// 1040 **L24** — TOTAL TAX = L22 + L23.
    pub total_tax: Usd,
    /// §6413(c) **excess Social Security** credit → Schedule 3 line 11 — per person `max(0, Σ box4 − MAX)`
    /// (MAX = 6.2% × the year's SS wage base), summed over taxpayer + spouse (never pooled). A single
    /// employer over-withholding refuses upstream (`SingleEmployerExcessSs`), so each box4 ≤ MAX here.
    pub excess_social_security: Usd,
    /// 1040 **L25a** — federal income tax withheld from Form(s) W-2 (Σ box 2).
    pub withholding_25a: Usd,
    /// 1040 **L25b** — federal income tax withheld from Form(s) 1099 (Σ box 4, across INT/DIV/G).
    pub withholding_25b: Usd,
    /// 1040 **L25** — total withholding = 25a (Σ W-2 box2) + 25b (Σ 1099 box4) + 25c (Form 8959 Part V +
    /// other withholding).
    pub total_withholding: Usd,
    /// 1040 **L33** — total payments = withholding (L25) + estimated (L26) + Schedule 3 L15 (extension L10
    /// + excess-SS L11). L36 apply-to-next-year is pinned 0/blank in v1.
    pub total_payments: Usd,
    /// 1040 **L34 → L35a** — overpayment refunded (0 when the return owes). At most one of this and
    /// `amount_owed` is nonzero (both are 0 when payments exactly equal the tax).
    pub overpayment_refund: Usd,
    /// 1040 **L37** — amount owed (0 when the return is due a refund).
    pub amount_owed: Usd,
    /// The inputs the P6 **printed chains** need that no other field carries — captured HERE, at the one
    /// derivation, so `assemble_printed_return` never re-sums anything. See [`PrintedInputs`].
    pub printed_inputs: PrintedInputs,
}

/// The Form 8959 / 8960 / 8995 inputs, captured at derivation.
///
/// These are not new facts — they are the *same* values `assemble_absolute` fed to the COMPUTED
/// `Form8959` / `Form8960` / `compute_8995`. Carrying them means the printed chain and the computed tax
/// see identical inputs by construction. The alternative (re-summing Σ box 5 inside the printed chain) is
/// a second derivation, and a second derivation is exactly how a filed form comes to disagree with the
/// tax the report computed from it (SPEC §3.1 — `btctax-forms` does no arithmetic, and neither should the
/// packet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrintedInputs {
    /// Form 8959 line 1 — Σ W-2 box 5 (household total Medicare wages).
    pub medicare_wages: Usd,
    /// Form 8959 Part V — Σ W-2 box 6 (Medicare tax withheld).
    pub medicare_withheld: Usd,
    /// Form 8960 line 7 — Σ non-business crypto **lending** interest (investment income with no home on
    /// 1040 line 2b; hobby mining/staking rewards stay OUT of NII).
    pub crypto_lending_interest: Usd,
    /// Form 8995 line 6 — Σ 1099-DIV box 5 (§199A REIT dividends).
    pub reit_dividends: Usd,
    /// Form 8995 line 7 — the REIT/PTP loss carryforward IN.
    pub reit_ptp_carryforward_in: Usd,
    /// Form 8995 line 11 — taxable income BEFORE the QBI deduction (AGI − L12).
    pub ti_before_qbi: Usd,
    /// Form 8995 line 12 — net capital gain (qualified dividends + §1(h) preferential net LTCG).
    pub qbi_net_capital_gain: Usd,
    /// Schedule 3 **line 10** — "Amount paid with request for extension to file". It is in the exact
    /// `total_payments`, so a printed chain that drops it tells the filer, ON THE FILED RETURN, to pay it
    /// a SECOND time (L31 falls ⇒ L37 "amount you owe" rises by the whole payment).
    pub extension_payment: Usd,
    /// 1040 page 1 — the digital-asset question. `true` iff the ledger shows digital-asset activity in
    /// the year (a disposal, recognized income, or a removal). btctax never answers **"No"**: a "No" it
    /// cannot vouch for is worse than leaving the question to the filer, so `false` means *unchecked*,
    /// not *answered in the negative*.
    pub digital_asset_activity: bool,
}

/// Assemble the absolute (WITH-crypto) 1040 from income through **total tax L24** for `year` (SPEC §5
/// stages 1–7). See [`AbsoluteReturn`]. Assumes `screen_inputs` + `screen_compute_dependent`
/// have passed (so all charitable classes are 50%-org and Schedule C net ≥ 0); the L12/L15 compute-dependent
/// refuses are checked afterward by [`screen_absolute`].
pub fn assemble_absolute(
    ri: &ReturnInputs,
    state: &LedgerState,
    params: &FullReturnParams,
    table: &TaxTable,
    year: i32,
) -> AbsoluteReturn {
    let status = ri.filing_status;

    // ── Schedule SE / §6017 block: reuse the FROZEN `compute_se_tax`, then drop it below the $400 floor.
    //    The two W-2 channels differ (deep/02 C4): the §1402(b)(1) SS cap uses the SE-earner's OWN box 3 +
    //    box 7 tips; Form 8959 uses HOUSEHOLD-total box 5 (identical to `derive_tax_profile`). ────────────
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
    let w2_medicare_wages: Usd = ri.w2s.iter().map(|w| w.box5_medicare_wages).sum();
    let schedule_c_expenses = ri
        .schedule_c
        .as_ref()
        .map(|c| c.expenses)
        .unwrap_or(Usd::ZERO);
    let se = compute_se_tax(
        state,
        year,
        status,
        table,
        w2_ss_wages,
        w2_medicare_wages,
        schedule_c_expenses,
    )
    .filter(|r| r.base >= SE_6017_FLOOR);
    let half_se = se.as_ref().map_or(Usd::ZERO, |r| r.deductible_half);

    // ── Income L1a..L9 (WITH crypto) ──────────────────────────────────────────────────────────────
    let wages = sum_wages(ri);
    let taxable_interest = sum_taxable_interest(ri);
    let ordinary_dividends = sum_ordinary_dividends(ri);
    let qualified_dividends = sum_qualified_dividends(ri);
    // §1222/§1211 capital netting computed ONCE: L7 (below) and the preferential slice (`net_ltcg`, → L16
    // / the QBI income limit) share this single `CapNet` (crypto Sch D + Σ box 2a, carryforward applied).
    let cap = capital_net(ri, state, year, status);
    let capital_gain = cap.ordinary_gain + cap.preferential_gain - cap.loss_deduction; // L7
    let net_ltcg = cap.preferential_gain; // §1(h) preferential net capital gain (≥ 0)

    // Schedule D, by its own lines. `net_1222` IS Schedule D: st_net is line 7, lt_net is line 15,
    // loss_deduction is line 21. Lines 6/14/21 are PAREN boxes ⇒ positive magnitudes.
    let sd_raw = crate::forms::schedule_d(state, year);
    let cf_in = ri.capital_loss_carryforward_in;
    let schedule_d = ScheduleDParts {
        st_proceeds_3d: sd_raw.st.proceeds,
        st_cost_3e: sd_raw.st.cost_basis,
        st_gain_3h: sd_raw.st.gain,
        st_carryover_6: cf_in.short,
        st_net_7: cap.st_net,
        lt_proceeds_10d: sd_raw.lt.proceeds,
        lt_cost_10e: sd_raw.lt.cost_basis,
        lt_gain_10h: sd_raw.lt.gain,
        cap_gain_distr_13: sum_cap_gain_distr(ri),
        lt_carryover_14: cf_in.long,
        lt_net_15: cap.lt_net,
        total_16: cap.st_net + cap.lt_net,
        loss_deduction_21: cap.loss_deduction,
        qualified_dividends,
    };

    // 1040 L8 = Sch 1 L10: state refund + Σ unemployment + Schedule C net (crypto business) + L8v
    // non-business crypto ordinary. Screening guarantees `business_se_gross ≥ expenses` here (no loss).
    let crypto = crypto_income(state, year);
    let schedule_c_net = (crypto.business_se_gross - schedule_c_expenses).max(Usd::ZERO);
    let schedule_1_income = ri.sch1.state_refund_taxable
        + sum_unemployment(ri)
        + schedule_c_net
        + crypto.nonbusiness_ordinary;

    let total_income =
        wages + taxable_interest + ordinary_dividends + capital_gain + schedule_1_income; // L9

    // ── Adjustments L10 (Sch 1 L26), AGI L11 ──────────────────────────────────────────────────────
    // §221 MAGI for the student-loan phase-out is AGI computed WITHOUT the student-loan deduction but WITH
    // ½-SE and the early-withdrawal penalty (Form 1040 / Sch 1 order).
    let early_wd: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box2_early_withdrawal_penalty)
        .sum();
    let agi_before_student_loan = total_income - early_wd - half_se;
    let student_loan = student_loan_deduction(
        ri.sch1.student_loan_interest_paid,
        agi_before_student_loan,
        status,
        params,
    );
    let adjustments = early_wd + half_se + student_loan;
    let agi = total_income - adjustments; // 1040 L11 (with-crypto AGI)

    // Schedule C exists only when the filer declared a crypto trade or business. Gross receipts are
    // the SE-eligible business crypto income; the net (floored at 0 — a loss refuses upstream) is the
    // SAME figure that feeds Schedule 1 line 3 and Schedule SE.
    let schedule_c = ri.schedule_c.as_ref().map(|_| ScheduleCParts {
        gross_receipts_1: crypto.business_se_gross,
        total_expenses_28: schedule_c_expenses,
        net_profit_31: schedule_c_net,
    });

    let schedule_1 = Schedule1Parts {
        state_refund_1: ri.sch1.state_refund_taxable,
        schedule_c_net_3: schedule_c_net,
        unemployment_7: sum_unemployment(ri),
        crypto_ordinary_8v: crypto.nonbusiness_ordinary,
        half_se_15: half_se,
        early_withdrawal_18: early_wd,
        student_loan_21: student_loan,
    };

    // ── Deductions L12–L15 (Schedule A on the WITH-crypto AGI, G7) ───────────────────────────────────
    // §63(c)(5) dependent floor uses the G21 with-crypto earned income = wages + Schedule C net − ½-SE
    // (now computable — completes `p3-m3-dependent-floor-earned-income-G21`; the derivation's non-crypto
    // side has no Schedule C, so it correctly stays wages-only). Earned income is a magnitude (≥ 0).
    let dependent_earned = (wages + schedule_c_net - half_se).max(Usd::ZERO);
    let standard = standard_deduction(ri, params, year, dependent_earned);

    // Absolute Schedule A charitable = user gifts + the ledger's §170(e) crypto donations, §170(b)-limited
    // at the with-crypto AGI. `apply_170b` runs UNCONDITIONALLY (even in a std-deduction year) so the
    // carryover ages (Reg. §1.170A-10(a)(2), G8) and `carryover_out` is the REAL filed carryover — the
    // `p3-carryover-writeback-P4` rider (ii) hoist out of the `schedule_a`-guard. All classes are 50%-org
    // (crypto → CapGainProp30/OrdinaryProp50; user gifts screened by `screen_inputs`), so `apply_170b`'s
    // precondition holds by construction — the rider (iii) requirement (this caller routes through the
    // refuse screens, per the function contract).
    let mut gifts = ri
        .schedule_a
        .as_ref()
        .map(|a| a.charitable.clone())
        .unwrap_or_default();
    gifts.extend(crypto_charitable_gifts(state, year));
    let charitable = apply_170b(agi, &gifts, &ri.charitable_carryover_in, year);
    let schedule_a = schedule_a_parts(ri, agi, &charitable, params);
    let itemized = schedule_a.map(|p| p.total_17);
    let deduction = choose_deduction(ri, standard, itemized); // L12
    let deduction_is_itemized = itemized_was_chosen(ri, standard, itemized);

    // QBI / Form 8995 (L13): REIT §199A dividends only (crypto Schedule C is NOT §199A QBI in v1). The
    // §199A(e)(2)-above-threshold refuse is compute-dependent → `screen_absolute` (this assembly is
    // infallible best-effort; the screen gates the report before the number is used).
    let reit_dividends: Usd = ri.div_1099.iter().map(|d| d.box5_section_199a).sum();
    let net_capital_gain = qualified_dividends + net_ltcg; // Form 8995 line 12
    let qbi = compute_8995(
        reit_dividends,
        ri.qbi.reit_ptp_carryforward_in,
        agi - deduction, // Form 8995 line 11 = TI before the QBI deduction
        net_capital_gain,
    );
    let total_deductions = deduction + qbi.deduction; // L14
    let taxable_income = (agi - total_deductions).max(Usd::ZERO); // L15

    // ── L16 regular tax (SPEC §5 stage 4 / §7.2 Schedule-D routing) ──────────────────────────────────
    // The Qualified Dividends & Capital Gain Tax Worksheet on the WITH-crypto TI (L15), qualified
    // dividends (L3a), and the §1(h) preferential net capital gain (`net_ltcg`). `qdcgt_line16` reduces
    // to the plain Tax Table / TCW when there is no preferential income, so it yields the correct L16 in
    // every §7.2 path (gain-both / ST-gain·LT-loss / net-loss-capped / zero) — the routing that differs
    // is *which worksheet the form shows* (a P6 fill concern), not the L16 value. The worksheet's
    // `min(L1, qd+ltcg)` cap is the `p2-pref-over-ti-clamp` fix (folds `p3-l16-absolute-P4`).
    let regular_tax = qdcgt_line16(
        table.ordinary_for(status),
        table.ltcg_for(status),
        taxable_income,
        qualified_dividends,
        net_ltcg,
    );

    // ── Sch 2 other taxes (SPEC §5 stage 7) ─────────────────────────────────────────────────────────
    // SE tax → Sch 2 L4 = SS + Medicare (the 0.9% is unbundled to Form 8959 Part II). Form 8959 Part I
    // reads the HOUSEHOLD Σ box5 (already computed above for the SE channel) and box6; Part II = se.addl.
    let w2_medicare_withheld: Usd = ri.w2s.iter().map(|w| w.box6_medicare_withheld).sum();
    let se_tax_sch2_l4 = sch2_line4_se(se.as_ref());
    let additional_medicare =
        form_8959(status, w2_medicare_wages, w2_medicare_withheld, se.as_ref());
    // Absolute Form 8960: NII rebuilt from line items — full 3b dividends (NOT just qualified), 2b interest,
    // §1211-limited L7, and non-business crypto LENDING interest (hobby mining/staking rewards stay OUT of
    // NII); MAGI = AGI (fail-closed). Schedule C business income is §1411(c)(6)-excluded (never in NII).
    let niit = form_8960(
        status,
        taxable_interest,
        ordinary_dividends,
        capital_gain,
        crypto.nonbusiness_lending_interest,
        agi,
    );

    // ── Credits + total tax (SPEC §5 stages 5–7 tail) ───────────────────────────────────────────────
    // §904(j) foreign-tax credit → Sch 3 L1 (eligibility ≤ $300/$600 passive/1099 enforced by
    // `screen_inputs`; over the ceiling refuses). Nonrefundable → capped by the tax at L22.
    let foreign_tax_credit: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box6_foreign_tax)
        .chain(ri.div_1099.iter().map(|d| d.box7_foreign_tax))
        .sum();
    // CTC/ODC — conservative omission (§3.4): L19 = 0 (loud advisory surfaced at render, P5).
    let ctc_odc_credit = Usd::ZERO;
    // Sch 2 Part I: L1z (excess-APTC) = 0 (no input); L2 (AMT) = 0 for a computed return (a triggered AMT
    // screen refuses via `screen_absolute`). So 1040 L17 = 0 and L18 = L16.
    let l18 = regular_tax; // L16 + Sch 2 L3 (= 0)
    let nonrefundable_credits = ctc_odc_credit + foreign_tax_credit; // L21 = L19 + L20 (v1: FTC only)
    let tax_after_credits = (l18 - nonrefundable_credits).max(Usd::ZERO); // L22
                                                                          // Sch 2 Part II (L21) → 1040 L23 = SE (L4) + Additional Medicare (L11) + NIIT (L12).
    let schedule_2_other_taxes =
        se_tax_sch2_l4 + additional_medicare.additional_medicare_tax + niit.tax;
    let total_tax = tax_after_credits + schedule_2_other_taxes; // L24

    // ── Excess-SS + payments → refund/owed (SPEC §5 stages 8–9) ─────────────────────────────────────
    let excess_social_security = excess_social_security(ri, table);

    // 1040 L25 withholding: 25a Σ W-2 box2; 25b Σ 1099 box4 (INT/DIV/G); 25c Form 8959 Part V + other.
    let wh_25a: Usd = ri.w2s.iter().map(|w| w.box2_fed_withheld).sum();
    let wh_25b: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box4_fed_withheld)
        .chain(ri.div_1099.iter().map(|d| d.box4_fed_withheld))
        .chain(ri.g_1099.iter().map(|g| g.box4_fed_withheld))
        .sum();
    let wh_25c = additional_medicare.part5_withholding + ri.payments.other_withholding;
    let total_withholding = wh_25a + wh_25b + wh_25c; // L25
                                                      // L33 total payments = L25 + L26 estimated + Sch 3 L15 (L10 extension + L11 excess-SS).
    let total_payments = total_withholding
        + ri.payments.estimated_tax_payments
        + ri.payments.extension_payment
        + excess_social_security;
    // L34→L35a refund vs L37 owed (L36 apply-to-next pinned 0/blank in v1).
    let overpayment_refund = (total_payments - total_tax).max(Usd::ZERO);
    let amount_owed = (total_tax - total_payments).max(Usd::ZERO);

    AbsoluteReturn {
        wages,
        taxable_interest,
        ordinary_dividends,
        qualified_dividends,
        capital_gain,
        schedule_1_income,
        total_income,
        adjustments,
        half_se_deduction: half_se,
        agi,
        se,
        standard_deduction: standard,
        schedule_1,
        schedule_c,
        schedule_d,
        itemized_deduction: itemized,
        schedule_a,
        deduction,
        deduction_is_itemized,
        qbi_deduction: qbi.deduction,
        total_deductions,
        taxable_income,
        net_ltcg,
        charitable_carryover_out: charitable.carryover_out,
        qbi_reit_ptp_carryforward_out: qbi.reit_ptp_carryforward_out,
        regular_tax,
        se_tax_sch2_l4,
        additional_medicare,
        niit,
        foreign_tax_credit,
        ctc_odc_credit,
        tax_after_credits,
        schedule_2_other_taxes,
        total_tax,
        excess_social_security,
        withholding_25a: wh_25a,
        withholding_25b: wh_25b,
        total_withholding,
        total_payments,
        overpayment_refund,
        amount_owed,
        // The printed chains read THESE — the same values the computed 8959/8960/8995 above were fed.
        printed_inputs: PrintedInputs {
            medicare_wages: w2_medicare_wages,
            medicare_withheld: w2_medicare_withheld,
            crypto_lending_interest: crypto.nonbusiness_lending_interest,
            reit_dividends,
            reit_ptp_carryforward_in: ri.qbi.reit_ptp_carryforward_in,
            ti_before_qbi: agi - deduction,
            qbi_net_capital_gain: net_capital_gain,
            extension_payment: ri.payments.extension_payment,
            digital_asset_activity: digital_asset_activity(state, year),
        },
    }
}

/// The 1040's digital-asset question: did the taxpayer receive, sell, exchange, or otherwise dispose of
/// a digital asset during the year? Answered from the LEDGER — a disposal, recognized income, or a
/// removal (gift/donation) dated in `year`.
///
/// `false` means the box is left **unchecked**, NOT answered "No": btctax never answers "No" (§3.4 —
/// a "No" it cannot vouch for is worse than leaving the question to the filer).
fn digital_asset_activity(state: &LedgerState, year: i32) -> bool {
    state.disposals.iter().any(|d| d.disposed_at.year() == year)
        || state
            .income_recognized
            .iter()
            .any(|i| i.recognized_at.year() == year)
        || state.removals.iter().any(|r| r.removed_at.year() == year)
}

/// Screen the **assembled-return** refuse rows (SPEC §4.10) — those that need the computed deduction /
/// taxable income, so they run AFTER [`assemble_absolute`] (which is infallible). Complements
/// [`crate::tax::return_refuse::screen_inputs`] (input-screenable) and [`screen_compute_dependent`]
/// (income/ledger-dependent). Returns the FIRST [`Refusal`], or `None`.
///
/// Rows: (a) QBI present with taxable-income-before-QBI above the §199A(e)(2) threshold (the 8995-A
/// phase-in is unmodeled, §4.5); (b) the 2024 AMT screening worksheet says Form 6251 must be filed
/// (§4.11); (c) taxable income ≤ 0 WITH a capital-loss carryforward-in (the G22 §1211/§1212 Capital Loss
/// Carryover Worksheet edge). A refund-only TI≤0 filer with NO carryforward is NOT refused (tax = 0,
/// withholding refunded — the r5-narrowed rule).
pub fn screen_absolute(
    ri: &ReturnInputs,
    ar: &AbsoluteReturn,
    params: &FullReturnParams,
) -> Option<Refusal> {
    // (a) QBI above the §199A(e)(2) threshold → 8995-A phase-in unmodeled.
    let reit_dividends: Usd = ri.div_1099.iter().map(|d| d.box5_section_199a).sum();
    if qbi_over_threshold(
        reit_dividends,
        ri.qbi.reit_ptp_carryforward_in,
        ar.agi - ar.deduction, // TI before QBI (Form 8995 line 11)
        ri.filing_status,
        params,
    ) {
        return refusal(
            RefuseReason::QbiAboveThreshold,
            "taxable income before the QBI deduction is above the §199A(e)(2) threshold — the Form 8995-A \
             phase-in (SSTB / wage-and-UBIA limits) is out of scope for v1",
        );
    }

    // (b) AMT screen — the 2024 "Should I fill in Form 6251?" worksheet (§4.11). Sch 2 L1z = 0 (no
    // excess-APTC input in v1); worksheet line 4 = Schedule 1 L1 taxable refund (no L8z input).
    if amt_should_file_6251(
        ri.filing_status,
        ar.agi,
        ar.qbi_deduction,
        ri.sch1.state_refund_taxable,
        ar.regular_tax,
        Usd::ZERO, // Schedule 2 line 1z (excess-APTC total) — unrepresentable in v1
        &params.amt,
    ) {
        return refusal(
            RefuseReason::AmtScreenTriggered,
            "the Form 6251 screening worksheet indicates you may owe alternative minimum tax — v1 does not \
             compute Form 6251, so the return is refused rather than understate the tax",
        );
    }

    // (c) Taxable income ≤ 0 with a capital-loss carryforward-in (the §1211/§1212 carryover-worksheet edge).
    let cf = ri.capital_loss_carryforward_in;
    if ar.taxable_income == Usd::ZERO && (cf.short > Usd::ZERO || cf.long > Usd::ZERO) {
        return refusal(
            RefuseReason::TaxableIncomeNonPositiveWithCarryforward,
            "taxable income is zero or negative with a capital-loss carryforward — the §1211/§1212 Capital \
             Loss Carryover Worksheet (which decides how much loss survives) is unmodeled in v1",
        );
    }

    None
}

/// Apply the **§4 R3-M6 carryover write-back**: stamp the absolute return's computed charitable +
/// QBI-REIT/PTP carryover-OUTs into `next_year`'s (year Y+1's) carryover-IN fields, provenance `Computed`.
/// Returns the updated `next_year` to persist, or `Err(message)` when it would silently overwrite a
/// **User**-provenance carryover (from `income import`) and `force` is false — never clobbers a user entry.
/// Both conflicts are checked BEFORE either field is written (atomic — a QBI conflict doesn't leave a
/// half-applied charitable write). A computed (or empty) existing carryover-in is overwritten silently.
pub fn apply_carryover_writeback(
    ar: &AbsoluteReturn,
    mut next_year: ReturnInputs,
    force: bool,
) -> Result<ReturnInputs, String> {
    if !force {
        if next_year
            .charitable_carryover_in
            .iter()
            .any(|c| c.provenance == CarryProvenance::User)
        {
            return Err(
                "next year's charitable carryover was user-entered (`income import`) — pass `--force` to \
                 overwrite it with the computed carryover"
                    .to_string(),
            );
        }
        if next_year.qbi.reit_ptp_carryforward_in > Usd::ZERO
            && next_year.qbi.reit_ptp_carryforward_in_provenance == CarryProvenance::User
        {
            return Err(
                "next year's QBI REIT/PTP carryforward was user-entered — pass `--force` to overwrite"
                    .to_string(),
            );
        }
    }
    next_year.charitable_carryover_in = ar
        .charitable_carryover_out
        .iter()
        .map(|c| CharitableCarryItem {
            provenance: CarryProvenance::Computed,
            ..c.clone()
        })
        .collect();
    next_year.qbi.reit_ptp_carryforward_in = ar.qbi_reit_ptp_carryforward_out;
    next_year.qbi.reit_ptp_carryforward_in_provenance = CarryProvenance::Computed;
    Ok(next_year)
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
    // `Person` is a TEST-only import now: the §63(f) box count moved to `packet::AgedBlindBoxes`, which
    // is the single source L12 consumes (`p6-aged-blind-checkboxes-missing`).
    use crate::tax::return_inputs::Person;

    /// A CharitableResult with nothing allowed — for Schedule A tests that isolate the medical/SALT/
    /// mortgage lines. (Schedule A now takes the whole result, since its `allowed_cash`/`_noncash`/
    /// `_carryover` ARE the form's lines 11/12/13.)
    fn no_charity() -> crate::tax::charitable::CharitableResult {
        charity(Usd::ZERO)
    }

    /// A CharitableResult allowing `allowed`, all of it current-year CASH (Schedule A line 11).
    fn charity(allowed: Usd) -> crate::tax::charitable::CharitableResult {
        crate::tax::charitable::CharitableResult {
            allowed_cash: allowed,
            allowed_noncash: Usd::ZERO,
            allowed_carryover: Usd::ZERO,
            allowed,
            carryover_out: Vec::new(),
        }
    }
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
        bad.ordinary_taxable_income +=
            good.qualified_dividends_and_other_pref_income + good.other_net_capital_gain; // 246,800 → 257,800
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

    /// P4.0 — the absolute (WITH-crypto) income assembly cross-foots (L9 = Σ income lines; L11 = L9 − L10)
    /// and the crypto figures (Schedule C mining + non-business reward + box-2a distribution) all land on
    /// the return. ½-SE (Schedule 1 L15) is computed from the Schedule SE base and subtracted into AGI.
    #[test]
    fn absolute_income_assembly_crossfoots_with_crypto() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(50000), dec!(50000), dec!(50000))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(1000),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![
            mining(dec!(60000)),
            income(IncomeKind::Reward, false, dec!(2000)),
        ]);
        let table = synthetic_table(2024);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &table, 2024);

        assert_eq!(ar.wages, dec!(50000)); // L1a
        assert_eq!(ar.taxable_interest, dec!(1000)); // L2b
        assert_eq!(ar.ordinary_dividends, Usd::ZERO);
        assert_eq!(ar.capital_gain, dec!(3000)); // L7 — box-2a LT distribution
        assert_eq!(ar.schedule_1_income, dec!(62000)); // L8 = Sch C net 60,000 + non-business reward 2,000
        assert_eq!(ar.total_income, dec!(116000)); // L9 = 50,000 + 1,000 + 0 + 3,000 + 62,000
        assert_eq!(
            ar.total_income,
            ar.wages
                + ar.taxable_interest
                + ar.ordinary_dividends
                + ar.capital_gain
                + ar.schedule_1_income
        );
        // Schedule SE base = round_cents(60,000 × 0.9235); ½-SE flows into adjustments.
        let se = ar.se.as_ref().expect("SE tax present above the $400 floor");
        assert_eq!(se.base, dec!(55410.00));
        assert!(ar.half_se_deduction > Usd::ZERO);
        assert_eq!(ar.half_se_deduction, se.deductible_half); // Sch 1 L15 = Sch SE L13 (excludes the 0.9%)
        assert_eq!(ar.adjustments, ar.half_se_deduction); // no early-wd / student-loan here
                                                          // Cross-foot L11 = L9 − L10 (with-crypto AGI).
        assert_eq!(ar.agi, ar.total_income - ar.adjustments);
        assert_eq!(ar.agi, dec!(116000) - ar.half_se_deduction);
    }

    /// P4.0 / §6017 (R3-M3): net SE earnings (the 92.35%-factored base) below $400 ⇒ NO SE tax and NO ½-SE,
    /// but the Schedule C net still counts as income. Above the floor, the SE result and ½-SE appear.
    #[test]
    fn absolute_se_respects_the_6017_400_floor() {
        let table = synthetic_table(2024);
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        // Gross $400 → base = round_cents(400 × 0.9235) = 369.40 < 400 ⇒ floored: no SE tax, no ½-SE.
        let below = assemble_absolute(
            &ri,
            &state_income(vec![mining(dec!(400))]),
            &ty2024_params(),
            &table,
            2024,
        );
        assert!(below.se.is_none());
        assert_eq!(below.half_se_deduction, Usd::ZERO);
        assert_eq!(below.schedule_1_income, dec!(400)); // Schedule C net still counts as income
        assert_eq!(below.agi, dec!(400)); // no ½-SE to subtract
                                          // Gross $500 → base = 461.75 ≥ 400 ⇒ SE tax + ½-SE present.
        let above = assemble_absolute(
            &ri,
            &state_income(vec![mining(dec!(500))]),
            &ty2024_params(),
            &table,
            2024,
        );
        assert!(above.se.is_some());
        assert!(above.half_se_deduction > Usd::ZERO);
        assert_eq!(above.agi, dec!(500) - above.half_se_deduction);
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
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
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
        assert_eq!(
            a.ordinary_taxable_income - b.ordinary_taxable_income,
            dec!(7000)
        );
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
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
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
        assert_eq!(
            standard_deduction(&filer(FilingStatus::Single), &p, 2024, Usd::ZERO),
            dec!(14600)
        );
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
        assert_eq!(
            standard_deduction(&mk(Some(date!(1960 - 01 - 01))), &p, 2024, Usd::ZERO),
            dec!(16550)
        );
        // Born 1960-01-02 → NOT aged.
        assert_eq!(
            standard_deduction(&mk(Some(date!(1960 - 01 - 02))), &p, 2024, Usd::ZERO),
            dec!(14600)
        );
        // None DOB → not established → NOT aged (conservative, fail-closed — dob-option-pin).
        assert_eq!(
            standard_deduction(&mk(None), &p, 2024, Usd::ZERO),
            dec!(14600)
        );
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
            schedule_a_deduction(
                &filer(FilingStatus::Single),
                dec!(100000),
                &no_charity(),
                &ty2024_params()
            ),
            None
        );
    }

    /// Medical over the 7.5% floor + SALT (income path) capped at $10k + mortgage.
    #[test]
    fn schedule_a_medical_floor_salt_cap_mortgage() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            medical: dec!(10000), // − 7.5%·100k = $2,500 allowed
            salt_state_estimated_payments: dec!(5000),
            salt_real_estate: dec!(8000), // 5d = 5,000 + 8,000 = 13,000 → capped $10,000
            mortgage_interest_1098: dec!(12000),
            ..Default::default()
        });
        // $2,500 + $10,000 + $12,000 + $0 charitable = $24,500.
        assert_eq!(
            schedule_a_deduction(&r, dec!(100000), &no_charity(), &ty2024_params()),
            Some(dec!(24500))
        );
    }

    /// Review M1 / r2 N1: a negative AGI is clamped to zero for the 7.5% medical floor, so the medical
    /// deduction is the FULL expense (no floor reduction) but is NEVER inflated ABOVE it. Without the clamp
    /// `medical − 7.5%·(−10,000) = medical + 750` would over-deduct.
    #[test]
    fn schedule_a_medical_floor_clamps_negative_agi() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            medical: dec!(10000),
            ..Default::default()
        });
        // agi.max(0) = 0 ⇒ floor = 0 ⇒ medical = $10,000 exactly (not $10,750).
        assert_eq!(
            schedule_a_deduction(&r, dec!(-10000), &no_charity(), &ty2024_params()),
            Some(dec!(10000))
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
            schedule_a_deduction(&r, dec!(100000), &charity(dec!(1000)), &ty2024_params()),
            Some(dec!(8000))
        );
        // MFS: $20,000 real-estate tax caps at $5,000.
        let mut mfs = filer(FilingStatus::Mfs);
        mfs.schedule_a = Some(ScheduleAInputs {
            salt_real_estate: dec!(20000),
            ..Default::default()
        });
        assert_eq!(
            schedule_a_deduction(&mfs, dec!(100000), &no_charity(), &ty2024_params()),
            Some(dec!(5000))
        );
    }

    /// `derive_tax_profile` takes max(standard, itemized): a big Schedule A beats the standard deduction.
    #[test]
    fn derive_uses_max_of_std_and_itemized() {
        let p = ty2024_params();
        let mut r = filer(FilingStatus::Single);
        r.w2s = vec![w2(
            Owner::Taxpayer,
            dec!(200000),
            dec!(200000),
            dec!(200000),
        )];
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(30000),
            salt_real_estate: dec!(15000), // capped at $10k
            ..Default::default()
        });
        // Itemized $40,000 > std $14,600 → taxable = $200,000 − $40,000 = $160,000.
        assert_eq!(
            schedule_a_deduction(&r, dec!(200000), &no_charity(), &p).unwrap(),
            dec!(40000)
        );
        assert_eq!(
            derive_tax_profile(&r, &p, 2024).ordinary_taxable_income,
            dec!(160000)
        );
    }

    /// §63(e) `ForceItemize` uses Schedule A even when it is smaller than the standard deduction.
    #[test]
    fn force_itemize_uses_schedule_a_even_when_smaller() {
        use crate::tax::return_inputs::ItemizeElection;
        let mut r = filer(FilingStatus::Single);
        r.w2s = vec![w2(
            Owner::Taxpayer,
            dec!(100000),
            dec!(100000),
            dec!(100000),
        )];
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(1000),
            ..Default::default()
        });
        r.itemize_election = ItemizeElection::ForceItemize;
        // Forced $1,000 (< std $14,600) → taxable = $100,000 − $1,000 = $99,000.
        assert_eq!(
            derive_tax_profile(&r, &ty2024_params(), 2024).ordinary_taxable_income,
            dec!(99000)
        );
    }

    /// §63(c)(6): an MFS filer whose spouse itemizes gets NO standard deduction.
    #[test]
    fn mfs_spouse_itemizes_forces_zero_std() {
        let p = ty2024_params();
        let mut r = filer(FilingStatus::Mfs);
        r.w2s = vec![w2(Owner::Taxpayer, dec!(50000), dec!(50000), dec!(50000))];
        r.mfs_spouse_itemizes = Some(true); // spouse itemizes → std = 0, no Sch A → taxable = $50,000.
        assert_eq!(
            derive_tax_profile(&r, &p, 2024).ordinary_taxable_income,
            dec!(50000)
        );
        // Spouse does NOT itemize → MFS std $14,600 → taxable = $35,400.
        r.mfs_spouse_itemizes = Some(false);
        assert_eq!(
            derive_tax_profile(&r, &p, 2024).ordinary_taxable_income,
            dec!(35400)
        );
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
        assert_eq!(
            screened(&dependent(dec!(3000)), &empty),
            Some(RefuseReason::KiddieTax)
        );
        // $2,000 interest ≤ $2,600 → no refusal.
        assert_eq!(screened(&dependent(dec!(2000)), &empty), None);
        // Non-business (hobby) crypto reward counts as unearned too: $2,000 interest + $1,000 reward > $2,600.
        let hobby = state_income(vec![income(IncomeKind::Reward, false, dec!(1000))]);
        assert_eq!(
            screened(&dependent(dec!(2000)), &hobby),
            Some(RefuseReason::KiddieTax)
        );
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

    // ── Absolute deductions L12–L15 (Phase 4 task 1) ─────────────────────────────────────────────
    use crate::event::BasisSource;
    use crate::identity::LotId;
    use crate::state::{Removal, RemovalLeg};

    /// A single §170 Donation removal leg in `year`, with a chosen holding-period `term`.
    fn donation_leg(term: Term, basis: Usd, fmv: Usd) -> RemovalLeg {
        RemovalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(1),
                split_sequence: 0,
            },
            sat: 100_000_000,
            basis,
            fmv_at_transfer: fmv,
            term,
            basis_source: BasisSource::ExchangeProvided,
            acquired_at: date!(2020 - 01 - 01),
            pseudo: false,
        }
    }
    fn donation(removed: Date, legs: Vec<RemovalLeg>) -> Removal {
        // §170(e): LT leg deducts FMV; ST leg deducts min(FMV, basis).
        let claimed: Usd = legs
            .iter()
            .map(|l| match l.term {
                Term::LongTerm => l.fmv_at_transfer,
                Term::ShortTerm => l.fmv_at_transfer.min(l.basis),
            })
            .sum();
        Removal {
            event: EventId::decision(1),
            kind: RemovalKind::Donation,
            removed_at: removed,
            legs,
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: Some(claimed),
            donee: None,
        }
    }
    fn state_removals(removals: Vec<Removal>) -> LedgerState {
        LedgerState {
            removals,
            ..Default::default()
        }
    }
    fn empty_ledger() -> LedgerState {
        LedgerState::default()
    }

    /// A LONG-term crypto donation from the ledger lands on the ABSOLUTE Schedule A at **FMV** (the
    /// `CapGainProp30` class), under the with-crypto-AGI 30% ceiling — the `p3-crypto-donation-delta-
    /// integration` P4 requirement (the derive-side profile excludes it; the absolute return includes it).
    #[test]
    fn absolute_schedule_a_includes_lt_crypto_donation_at_fmv() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(200000),
                dec!(160000),
                dec!(200000),
            )],
            schedule_a: Some(ScheduleAInputs {
                mortgage_interest_1098: dec!(5000),
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_removals(vec![donation(
            date!(2024 - 06 - 01),
            vec![donation_leg(Term::LongTerm, dec!(10000), dec!(40000))],
        )]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &synthetic_table(2024), 2024);
        // AGI = wages $200,000 (a donation recognizes no gain — no crypto income).
        assert_eq!(ar.agi, dec!(200000));
        // Sch A = mortgage $5,000 + LT crypto FMV $40,000 (≤ 30% ceiling min(60k,100k)=60k) = $45,000.
        assert_eq!(ar.itemized_deduction, Some(dec!(45000)));
        assert_eq!(ar.deduction, dec!(45000)); // > std $14,600
        assert_eq!(ar.taxable_income, dec!(155000)); // 200,000 − 45,000
    }

    /// A SHORT-term crypto donation deducts the §170(e) **basis** `min(FMV, basis)` (the `OrdinaryProp50`
    /// class) — NOT FMV. FMV $30,000 / basis $12,000 ⇒ $12,000 on Schedule A.
    #[test]
    fn absolute_schedule_a_short_term_crypto_donation_uses_basis() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(200000),
                dec!(160000),
                dec!(200000),
            )],
            schedule_a: Some(ScheduleAInputs {
                mortgage_interest_1098: dec!(5000),
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_removals(vec![donation(
            date!(2024 - 06 - 01),
            vec![donation_leg(Term::ShortTerm, dec!(12000), dec!(30000))],
        )]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &synthetic_table(2024), 2024);
        // Sch A = mortgage $5,000 + ST §170(e) basis $12,000 (OrdinaryProp50, 50% ceiling) = $17,000.
        assert_eq!(ar.itemized_deduction, Some(dec!(17000)));
    }

    /// A crypto donation over the §170(b) 30% ceiling produces a `carryover_out` (the real filed
    /// carryover), and `apply_170b` runs even though the std deduction wins — the aging hoist (rider ii).
    #[test]
    fn crypto_donation_over_ceiling_carries_over_even_in_std_year() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            // No Schedule A → std deduction wins, but the carryover must still age (G8).
            ..Default::default()
        };
        let st = state_removals(vec![donation(
            date!(2024 - 06 - 01),
            vec![donation_leg(Term::LongTerm, dec!(20000), dec!(70000))],
        )]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &synthetic_table(2024), 2024);
        // No Schedule A ⇒ itemized None ⇒ std $14,600 taken.
        assert_eq!(ar.itemized_deduction, None);
        assert_eq!(ar.deduction, dec!(14600));
        // 30% ceiling on $100k AGI = $30,000 allowed; the $40,000 excess carries (2024 vintage).
        assert_eq!(
            ar.charitable_carryover_out,
            vec![CharitableCarryItem {
                class: CharitableClass::CapGainProp30,
                amount: dec!(40000),
                origin_year: 2024,
                provenance: crate::tax::return_inputs::CarryProvenance::default(),
            }]
        );
    }

    /// G21 (`p3-m3-dependent-floor-earned-income-G21`): the §63(c)(5) dependent std-deduction floor uses
    /// the with-crypto earned income = wages + Schedule C net − ½-SE (now computable), not wages alone.
    #[test]
    fn dependent_floor_uses_g21_with_crypto_earned_income() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        ri.header.can_be_claimed_as_dependent_taxpayer = true;
        let st = state_income(vec![mining(dec!(10000))]); // Sch C net $10,000, earned (not kiddie-unearned)
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &synthetic_table(2024), 2024);
        let half = ar.half_se_deduction;
        assert!(half > Usd::ZERO);
        // floor = min(basic $14,600, max($1,300, earned + $450)) with earned = 0 + 10,000 − ½-SE.
        assert_eq!(ar.standard_deduction, dec!(10450) - half);
        assert_eq!(ar.itemized_deduction, None); // no Schedule A
        assert_eq!(ar.deduction, dec!(10450) - half);
    }

    /// QBI/Form 8995 (L13): REIT §199A dividends reduce taxable income through L14 = L12 + L13.
    #[test]
    fn qbi_deduction_reduces_taxable_income() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(5000),    // includes the §199A subset (strip-once)
                box5_section_199a: dec!(5000), // REIT dividends
                ..Default::default()
            }],
            ..Default::default()
        };
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &synthetic_table(2024),
            2024,
        );
        // AGI = 100,000 + 5,000 ord div = 105,000; std 14,600; TI-before-QBI = 90,400.
        // QBI: 20% × 5,000 = 1,000; income limit 20% × 90,400 = 18,080 → L13 = 1,000.
        assert_eq!(ar.qbi_deduction, dec!(1000));
        assert_eq!(ar.total_deductions, dec!(15600)); // 14,600 + 1,000
        assert_eq!(ar.taxable_income, dec!(89400)); // 105,000 − 15,600
    }

    /// QBI above the §199A(e)(2) threshold (with QBI present) refuses via `screen_absolute` (8995-A
    /// unmodeled); the same high income with NO REIT dividends is not refused.
    #[test]
    fn qbi_above_threshold_refuses() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(250000),
                dec!(168600),
                dec!(250000),
            )],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(1000),
                box5_section_199a: dec!(1000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        // TI-before-QBI = 251,000 − 14,600 = 236,400 > 191,950 → refuse.
        assert_eq!(
            screen_absolute(&ri, &ar, &p).map(|r| r.reason),
            Some(RefuseReason::QbiAboveThreshold)
        );
        // Drop the REIT dividends → no QBI at all → no refuse even at the same high income.
        let mut no_qbi = ri.clone();
        no_qbi.div_1099[0].box5_section_199a = Usd::ZERO;
        let ar2 = assemble_absolute(&no_qbi, &empty_ledger(), &p, &table, 2024);
        assert_eq!(screen_absolute(&no_qbi, &ar2, &p), None);
    }

    /// TI ≤ 0 WITH a capital-loss carryforward-in refuses (the §1211/§1212 carryover-worksheet edge);
    /// the SAME zero-TI return with NO carryforward is a refund-only filer — NOT refused (r5-narrowed).
    #[test]
    fn taxable_income_nonpositive_with_carryforward_refuses() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(5000), dec!(5000), dec!(5000))],
            ..Default::default()
        };
        ri.capital_loss_carryforward_in = Carryforward {
            short: dec!(2000),
            long: Usd::ZERO,
        };
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        // AGI = 5,000 wages + L7(−2,000 §1211-limited carryforward loss) = 3,000; std 14,600 → TI = 0.
        assert_eq!(ar.taxable_income, Usd::ZERO);
        assert_eq!(
            screen_absolute(&ri, &ar, &p).map(|r| r.reason),
            Some(RefuseReason::TaxableIncomeNonPositiveWithCarryforward)
        );
        // No carryforward → still TI = 0, but a refund-only filer is NOT refused.
        let mut norf = ri.clone();
        norf.capital_loss_carryforward_in = Carryforward::default();
        let ar2 = assemble_absolute(&norf, &empty_ledger(), &p, &table, 2024);
        assert_eq!(ar2.taxable_income, Usd::ZERO);
        assert_eq!(screen_absolute(&norf, &ar2, &p), None);
    }

    // ── L16 regular tax + §7.2 Schedule-D routing (Phase 4 task 2) ────────────────────────────────
    use crate::state::{Disposal, DisposalLeg};
    use crate::tax::method::regular_tax;
    use crate::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule};

    /// A TaxTable carrying the REAL TY2024 **Single + MFJ** ordinary schedules + §1(h) LTCG breakpoints
    /// (Rev. Proc. 2023-34) so L16 values are cent-exact against the `method.rs`-proven QDCGT kernel; the SS
    /// wage base is the real TY2024 $168,600.
    fn real_2024_table() -> TaxTable {
        let mut ordinary = BTreeMap::new();
        ordinary.insert(
            FilingStatus::Single,
            OrdinarySchedule {
                brackets: vec![
                    OrdinaryBracket {
                        lower: dec!(0),
                        rate: dec!(0.10),
                    },
                    OrdinaryBracket {
                        lower: dec!(11600),
                        rate: dec!(0.12),
                    },
                    OrdinaryBracket {
                        lower: dec!(47150),
                        rate: dec!(0.22),
                    },
                    OrdinaryBracket {
                        lower: dec!(100525),
                        rate: dec!(0.24),
                    },
                    OrdinaryBracket {
                        lower: dec!(191950),
                        rate: dec!(0.32),
                    },
                    OrdinaryBracket {
                        lower: dec!(243725),
                        rate: dec!(0.35),
                    },
                    OrdinaryBracket {
                        lower: dec!(609350),
                        rate: dec!(0.37),
                    },
                ],
            },
        );
        ordinary.insert(
            FilingStatus::Mfj,
            OrdinarySchedule {
                brackets: vec![
                    OrdinaryBracket {
                        lower: dec!(0),
                        rate: dec!(0.10),
                    },
                    OrdinaryBracket {
                        lower: dec!(23200),
                        rate: dec!(0.12),
                    },
                    OrdinaryBracket {
                        lower: dec!(94300),
                        rate: dec!(0.22),
                    },
                    OrdinaryBracket {
                        lower: dec!(201050),
                        rate: dec!(0.24),
                    },
                    OrdinaryBracket {
                        lower: dec!(383900),
                        rate: dec!(0.32),
                    },
                    OrdinaryBracket {
                        lower: dec!(487450),
                        rate: dec!(0.35),
                    },
                    OrdinaryBracket {
                        lower: dec!(731200),
                        rate: dec!(0.37),
                    },
                ],
            },
        );
        let mut ltcg = BTreeMap::new();
        ltcg.insert(
            FilingStatus::Single,
            LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(518900),
            },
        );
        ltcg.insert(
            FilingStatus::Mfj,
            LtcgBreakpoints {
                max_zero: dec!(94050),
                max_fifteen: dec!(583750),
            },
        );
        TaxTable {
            year: 2024,
            source: "TEST-TY2024-Single",
            ordinary,
            ltcg,
            gift_annual_exclusion: dec!(18000),
            ss_wage_base: dec!(168600),
            gift_lifetime_exclusion: dec!(13_610_000),
        }
    }
    fn disp_leg(term: Term, proceeds: Usd, basis: Usd) -> DisposalLeg {
        DisposalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(1),
                split_sequence: 0,
            },
            sat: 100_000_000,
            proceeds,
            basis,
            gain: proceeds - basis,
            term,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: date!(2020 - 01 - 01),
            wallet: crate::identity::WalletId::SelfCustody { label: "w".into() },
            pseudo: false,
        }
    }
    fn state_disposals(legs: Vec<DisposalLeg>) -> LedgerState {
        LedgerState {
            disposals: vec![Disposal {
                event: EventId::decision(1),
                kind: crate::event::DisposeKind::Sell,
                disposed_at: date!(2024 - 05 - 01),
                legs,
                fee_mini_disposition: false,
            }],
            ..Default::default()
        }
    }

    /// §7.2 path — a net LT gain (box-2a capital-gain distribution) → QDCGT. TI 120,000 / net-LTCG 20,000
    /// ⇒ L16 = $20,053 (deep/01 example b, cent-exact through the assembly).
    fn wages_single(wages: Usd) -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, wages, wages, wages)],
            ..Default::default()
        }
    }
    #[test]
    fn l16_lt_gain_uses_qdcgt() {
        let mut ri = wages_single(dec!(114600));
        ri.div_1099 = vec![Form1099Div {
            box2a_capgain_distr: dec!(20000), // LT-character → preferential net LTCG
            ..Default::default()
        }];
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.taxable_income, dec!(120000)); // AGI 134,600 − std 14,600
        assert_eq!(ar.net_ltcg, dec!(20000));
        assert_eq!(ar.regular_tax, dec!(20053)); // QDCGT (deep/01 ex. b)
    }

    /// §7.2 path — qualified dividends but no net LTCG (an ST-gain/LT-loss-style year) still routes to
    /// QDCGT (preferential rate on the QD). TI 60,000 / QD 2,000 ⇒ L16 = $8,119 (deep/01 example c).
    #[test]
    fn l16_qualified_dividends_use_qdcgt() {
        let mut ri = wages_single(dec!(72600));
        ri.div_1099 = vec![Form1099Div {
            box1a_ordinary: dec!(2000),
            box1b_qualified: dec!(2000),
            ..Default::default()
        }];
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.taxable_income, dec!(60000)); // AGI 74,600 − std 14,600
        assert_eq!(ar.qualified_dividends, dec!(2000));
        assert_eq!(ar.net_ltcg, Usd::ZERO);
        assert_eq!(ar.regular_tax, dec!(8119)); // QDCGT (deep/01 ex. c)
    }

    /// §7.2 path — NO preferential income → L16 collapses to the plain Tax Table (QDCGT ≡ `regular_tax`).
    #[test]
    fn l16_no_preferential_income_is_tax_table() {
        let ri = wages_single(dec!(60000));
        let table = real_2024_table();
        let ar = assemble_absolute(&ri, &empty_ledger(), &ty2024_params(), &table, 2024);
        assert_eq!(ar.taxable_income, dec!(45400)); // 60,000 − 14,600
        assert_eq!(ar.qualified_dividends, Usd::ZERO);
        assert_eq!(ar.net_ltcg, Usd::ZERO);
        // Identical to the plain Tax Table on the same TI — no QDCGT preferential branch taken.
        assert_eq!(
            ar.regular_tax,
            regular_tax(table.ordinary_for(FilingStatus::Single), dec!(45400))
        );
    }

    /// §7.2 path — a net-loss year: the §1211-capped −$3,000 reaches L7, the preferential slice is 0, and
    /// L16 is the Tax Table on the loss-reduced TI (deep/01 loss-year shape).
    #[test]
    fn l16_net_loss_capped_path() {
        let mut ri = wages_single(dec!(60000));
        ri.capital_loss_carryforward_in = Carryforward {
            short: dec!(5000),
            long: Usd::ZERO,
        };
        let table = real_2024_table();
        let ar = assemble_absolute(&ri, &empty_ledger(), &ty2024_params(), &table, 2024);
        assert_eq!(ar.capital_gain, dec!(-3000)); // §1211 limit
        assert_eq!(ar.net_ltcg, Usd::ZERO);
        assert_eq!(ar.taxable_income, dec!(42400)); // (60,000 − 3,000) − 14,600
        assert_eq!(
            ar.regular_tax,
            regular_tax(table.ordinary_for(FilingStatus::Single), dec!(42400))
        );
    }

    /// §7.2 path — ST gain cross-netted against an LT loss (Schedule D line 16 netting): the surviving
    /// net is SHORT-term (ordinary), so L7 > 0 but the preferential slice is 0. ST $10,000 gain − LT
    /// $4,000 loss ⇒ L7 = $6,000 ordinary, net-LTCG 0.
    #[test]
    fn l16_short_gain_long_loss_cross_nets_to_ordinary() {
        let ri = wages_single(dec!(50000));
        let st = state_disposals(vec![
            disp_leg(Term::ShortTerm, dec!(30000), dec!(20000)), // +10,000 ST
            disp_leg(Term::LongTerm, dec!(6000), dec!(10000)),   // −4,000 LT
        ]);
        let table = real_2024_table();
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &table, 2024);
        assert_eq!(ar.capital_gain, dec!(6000)); // 10,000 ST − 4,000 LT cross-net → ordinary
        assert_eq!(ar.net_ltcg, Usd::ZERO);
        assert_eq!(ar.taxable_income, dec!(41400)); // (50,000 + 6,000) − 14,600
        assert_eq!(
            ar.regular_tax,
            regular_tax(table.ordinary_for(FilingStatus::Single), dec!(41400))
        );
    }

    /// `p2-pref-over-ti-clamp` on the absolute side: preferential income exceeding taxable income is CAPPED
    /// at TI (the QDCGT `min(L1, qd+ltcg)`), so L16 is not overstated. TI 35,400 / QD 50,000 ⇒ L16 = $0
    /// (method.rs KAT-1 — the uncapped worksheet would wrongly produce $446).
    #[test]
    fn l16_preferential_over_ti_is_capped() {
        let mut ri = wages_single(Usd::ZERO);
        ri.w2s.clear(); // no wages
        ri.div_1099 = vec![Form1099Div {
            box1a_ordinary: dec!(50000),
            box1b_qualified: dec!(50000),
            ..Default::default()
        }];
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.taxable_income, dec!(35400)); // 50,000 − 14,600
        assert_eq!(ar.qualified_dividends, dec!(50000));
        assert_eq!(ar.regular_tax, Usd::ZERO); // capped → $0 (not $446)
    }

    /// r2 Nit — the dual-report deduction label reflects the actual §63(e) election, not an amount
    /// heuristic: `ForceItemize` is "itemized" even with no Schedule A ($0 deduction); `Auto` with no
    /// Schedule A is "standard".
    #[test]
    fn deduction_is_itemized_reflects_the_election() {
        use crate::tax::return_inputs::ItemizeElection;
        let params = ty2024_params();
        let table = real_2024_table();
        let mut force = wages_single(dec!(60000));
        force.itemize_election = ItemizeElection::ForceItemize;
        let ar = assemble_absolute(&force, &empty_ledger(), &params, &table, 2024);
        assert!(ar.deduction_is_itemized); // labeled itemized even though...
        assert_eq!(ar.deduction, Usd::ZERO); // ...§63(e) forced-itemize with nothing to itemize is $0
        let ar2 = assemble_absolute(
            &wages_single(dec!(60000)),
            &empty_ledger(),
            &params,
            &table,
            2024,
        );
        assert!(!ar2.deduction_is_itemized); // Auto, no Schedule A → standard
        assert_eq!(ar2.deduction, dec!(14600));
    }

    // ── Sch 2 other taxes wired into the absolute assembly (Phase 4 task 3/5) ─────────────────────

    /// Absolute Form 8960 NII uses the FULL 1040 3b dividends (not just qualified — the key absolute-vs-
    /// delta distinction) + 2b interest + non-business crypto LENDING interest, while a hobby mining
    /// REWARD is excluded from NII (it is Sch 1 L8v income, not investment income).
    #[test]
    fn absolute_niit_full_dividends_lending_in_reward_out() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(250000),
                dec!(168600),
                dec!(250000),
            )],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000), // full 3b
                box1b_qualified: dec!(4000), // only part is qualified
                ..Default::default()
            }],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let st = state_income(vec![
            income(IncomeKind::Reward, false, dec!(5000)), // hobby reward → NOT NII (Sch 1 L8v only)
            income(IncomeKind::Interest, false, dec!(2000)), // non-business lending interest → NII
        ]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        // NII = 2b 3,000 + 3b 10,000 (FULL box1a) + L7 0 + crypto lending 2,000 = 15,000 (reward excluded).
        assert_eq!(ar.niit.nii, dec!(15000));
        // AGI = 250,000 + 3,000 + 10,000 + (reward 5,000 + lending 2,000 on L8v) = 270,000 → over 70,000.
        assert_eq!(ar.niit.magi, dec!(270000));
        assert_eq!(ar.niit.tax, dec!(570.00)); // 3.8% × 15,000
    }

    /// Absolute SE tax unbundles into the assembly: Sch 2 L4 = SS + Medicare (NOT the total), and the
    /// §1401(b)(2) 0.9% lands on Form 8959 Part II. A $300k mining fixture makes `addl` > 0 (discriminating).
    #[test]
    fn absolute_se_unbundles_to_sch2_l4_and_8959_part2() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![mining(dec!(300000))]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        let se = ar.se.as_ref().expect("SE tax present");
        assert!(se.addl > Usd::ZERO);
        assert_eq!(ar.se_tax_sch2_l4, se.ss + se.medicare); // Sch 2 L4 excludes the 0.9%
        assert_ne!(ar.se_tax_sch2_l4, se.total); // discriminating
        assert_eq!(ar.additional_medicare.part2_se, se.addl); // 0.9% routed to Form 8959 Part II
        assert_eq!(ar.additional_medicare.additional_medicare_tax, se.addl); // no wages → Part I 0
    }

    /// Form 8959 Part I reads the HOUSEHOLD Σ box5 (summed across W-2s), not a single employer's.
    #[test]
    fn absolute_8959_part1_sums_household_medicare_wages() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2(Owner::Taxpayer, dec!(150000), dec!(150000), dec!(150000)),
                w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000)),
            ],
            ..Default::default()
        };
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        // Σ box5 = 250,000 > 200,000 Single threshold → Part I = 0.9% × 50,000 = $450.
        assert_eq!(ar.additional_medicare.part1_wages, dec!(450.00));
    }

    /// The AMT screen (§4.11) wired through `screen_absolute`: a very-high-income filer (worksheet line-12
    /// STOP, AMTI − exemption > $232,600) is REFUSED; a common household clears (Sch 2 line 2 = 0).
    #[test]
    fn amt_screen_refuses_high_income_clears_common() {
        let p = ty2024_params();
        let table = real_2024_table();
        // $900k wages → worksheet line 11 ≈ 887k > 232,600 → fill 6251 → refuse.
        let high = wages_single(dec!(900000));
        let ar_high = assemble_absolute(&high, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            screen_absolute(&high, &ar_high, &p).map(|r| r.reason),
            Some(RefuseReason::AmtScreenTriggered)
        );
        // $150k wages → line 11 = 64,300 ≤ 232,600 and 26% × it < regular tax → cleared (no refuse).
        let common = wages_single(dec!(150000));
        let ar_common = assemble_absolute(&common, &empty_ledger(), &p, &table, 2024);
        assert_eq!(screen_absolute(&common, &ar_common, &p), None);
    }

    // ── Credits + total tax L24 (Phase 4 task 2/6/7) ─────────────────────────────────────────────

    /// KAT-16 — §904(j) foreign-tax credit = Σ(1099-INT box6 + 1099-DIV box7) → Schedule 3 line 1, and it
    /// reduces the income tax after credits (L22).
    #[test]
    fn foreign_tax_credit_on_schedule_3_line_1() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(5000),
                box6_foreign_tax: dec!(120),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(3000),
                box7_foreign_tax: dec!(80),
                ..Default::default()
            }],
            ..Default::default()
        };
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.foreign_tax_credit, dec!(200)); // 120 + 80 (≤ $300 ceiling, screened)
        assert_eq!(ar.tax_after_credits, ar.regular_tax - dec!(200)); // L22 = L16 − FTC (no other credits)
    }

    /// CTC/ODC is a conservative omission (§3.4): 1040 L19 = 0 even with dependents (the loud advisory is
    /// surfaced at render, P5). The tax is never reduced by a CTC → overstates at worst, never understates.
    #[test]
    fn ctc_odc_conservatively_omitted_l19_zero() {
        let mut ri = wages_single(dec!(60000));
        ri.header.dependents = vec![crate::tax::return_inputs::Dependent {
            name: "Child".into(),
            relationship: "son".into(),
            date_of_birth: Some(date!(2015 - 01 - 01)),
            ..Default::default()
        }];
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.ctc_odc_credit, Usd::ZERO);
        assert_eq!(ar.tax_after_credits, ar.regular_tax); // no FTC, no CTC → L22 = L16
    }

    /// Total tax L24 = L22 (income tax after credits) + L23 (Sch 2 Part II other taxes = SE + 8959 + 8960).
    /// A fixture with SE income, NIIT, and an FTC exercises every summand at once.
    #[test]
    fn total_tax_l24_composition() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(200000),
                dec!(168600),
                dec!(200000),
            )],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(5000),
                box6_foreign_tax: dec!(100),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(3000),
                box7_foreign_tax: dec!(50),
                ..Default::default()
            }],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![mining(dec!(60000))]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        // Every summand is live: FTC $150, SE tax > 0, NIIT on $8,000 investment income (MAGI well over
        // $200k) = 3.8% × 8,000 = $304.
        assert_eq!(ar.foreign_tax_credit, dec!(150));
        assert!(ar.se.is_some() && ar.se_tax_sch2_l4 > Usd::ZERO);
        assert_eq!(ar.niit.tax, dec!(304.00));
        // Composition identities (L23, L22, L24).
        assert_eq!(
            ar.schedule_2_other_taxes,
            ar.se_tax_sch2_l4 + ar.additional_medicare.additional_medicare_tax + ar.niit.tax
        );
        assert_eq!(
            ar.tax_after_credits,
            (ar.regular_tax - ar.foreign_tax_credit).max(Usd::ZERO)
        );
        assert_eq!(
            ar.total_tax,
            ar.tax_after_credits + ar.schedule_2_other_taxes
        );
    }

    /// The FTC is NONREFUNDABLE: when it exceeds the income tax (L16), L22 floors at $0 and the excess is
    /// lost — never a refund of foreign tax.
    #[test]
    fn foreign_tax_credit_is_nonrefundable() {
        let mut ri = wages_single(dec!(17000));
        ri.int_1099 = vec![Form1099Int {
            box6_foreign_tax: dec!(300), // ≤ $300 ceiling
            ..Default::default()
        }];
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(ar.foreign_tax_credit, dec!(300));
        assert!(ar.regular_tax < dec!(300)); // TI $2,400 → L16 ≈ $241
        assert_eq!(ar.tax_after_credits, Usd::ZERO); // capped by tax; excess FTC not refundable
    }

    // ── Excess-SS + payments → refund/owed (Phase 4 task 6) ──────────────────────────────────────

    /// KAT-11 — §6413(c) excess Social Security is PER PERSON, never pooled. MAX = 6.2% × $168,600 =
    /// $10,453.20 (TY2024). Two employers → the excess is creditable; one employer nets 0; on a joint
    /// return each spouse's excess is computed separately (pooling would over-credit).
    #[test]
    fn excess_social_security_per_person_not_pooled() {
        let table = real_2024_table(); // ss_wage_base $168,600 → MAX $10,453.20
        let w2_ss = |owner: Owner, box4: Usd| W2 {
            owner,
            box4_ss_withheld: box4,
            ..Default::default()
        };
        // Single, two employers each $6,000 → Σ $12,000 > MAX → excess $1,546.80.
        let two = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000)),
                w2_ss(Owner::Taxpayer, dec!(6000)),
            ],
            ..Default::default()
        };
        assert_eq!(excess_social_security(&two, &table), dec!(1546.80));
        // One employer $6,000 (< MAX) → no excess.
        let one = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2_ss(Owner::Taxpayer, dec!(6000))],
            ..Default::default()
        };
        assert_eq!(excess_social_security(&one, &table), Usd::ZERO);
        // MFJ: taxpayer 2×$6,000 (excess $1,546.80) + spouse 1×$8,000 (< MAX → 0) → total $1,546.80,
        // NOT the pooled max(0, 20,000 − 10,453.20) = $9,546.80.
        let mfj = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000)),
                w2_ss(Owner::Taxpayer, dec!(6000)),
                w2_ss(Owner::Spouse, dec!(8000)),
            ],
            ..Default::default()
        };
        assert_eq!(excess_social_security(&mfj, &table), dec!(1546.80));
    }

    /// Total payments L33 sums every source: 25a (W-2 box2) + 25b (1099 box4) + 25c (8959 Part V + other)
    /// + estimated (L26) + extension + excess-SS (Sch 3).
    #[test]
    fn total_payments_sums_all_sources() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(100000),
                box2_fed_withheld: dec!(15000),
                box3_ss_wages: dec!(100000),
                box5_medicare_wages: dec!(100000),
                ..Default::default()
            }],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(5000),
                box4_fed_withheld: dec!(500),
                ..Default::default()
            }],
            payments: crate::tax::return_inputs::Payments {
                estimated_tax_payments: dec!(2000),
                extension_payment: dec!(1000),
                other_withholding: dec!(300),
            },
            ..Default::default()
        };
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        // 25a 15,000 + 25b 500 + 25c (8959 Part V 0 + other 300) = 15,800.
        assert_eq!(ar.total_withholding, dec!(15800));
        // + estimated 2,000 + extension 1,000 (+ excess-SS 0) = 18,800.
        assert_eq!(ar.total_payments, dec!(18800));
    }

    /// The return settles to a refund (payments > tax) or an amount owed (tax > payments) — exactly one is
    /// nonzero. L36 apply-to-next-year is pinned 0 (not modeled).
    #[test]
    fn settle_refund_or_owed() {
        let p = ty2024_params();
        let table = real_2024_table();
        let mk = |withheld: Usd| ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(60000),
                box2_fed_withheld: withheld,
                box3_ss_wages: dec!(60000),
                box5_medicare_wages: dec!(60000),
                ..Default::default()
            }],
            ..Default::default()
        };
        // Over-withheld → refund (TI $45,400 → total tax ≈ $5,219 < $12,000).
        let refund = assemble_absolute(&mk(dec!(12000)), &empty_ledger(), &p, &table, 2024);
        assert_eq!(refund.total_payments, dec!(12000));
        assert_eq!(refund.overpayment_refund, dec!(12000) - refund.total_tax);
        assert_eq!(refund.amount_owed, Usd::ZERO);
        // Under-withheld → owed.
        let owed = assemble_absolute(&mk(dec!(1000)), &empty_ledger(), &p, &table, 2024);
        assert_eq!(owed.amount_owed, owed.total_tax - dec!(1000));
        assert_eq!(owed.overpayment_refund, Usd::ZERO);
    }

    /// Phase-4 acceptance (Fable r1 I2 / KAT-12): deep/02 Example 2 — MFJ household with BOTH wage and SE
    /// Medicare channels + $60k business mining, the full Form 8959 Part I+II+V composing through
    /// `assemble_absolute`, cent-exact. Taxpayer box5 220,000 (box3 168,600 capped, box6 3,370) + spouse
    /// box5 60,000 (box3 60,000, box6 870); Schedule C net 60,000 → SE base 55,410.00.
    #[test]
    fn deep02_example2_other_taxes_block_to_the_cent() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(220000),
                    box3_ss_wages: dec!(168600), // SS cap already reached by wages
                    box5_medicare_wages: dec!(220000),
                    box6_medicare_withheld: dec!(3370),
                    ..Default::default()
                },
                W2 {
                    owner: Owner::Spouse,
                    box1_wages: dec!(60000),
                    box3_ss_wages: dec!(60000),
                    box5_medicare_wages: dec!(60000),
                    box6_medicare_withheld: dec!(870),
                    ..Default::default()
                },
            ],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer, // the SE earner (own box3 168,600 → SS cap fully used → ss = 0)
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![mining(dec!(60000))]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        let se = ar.se.as_ref().expect("SE tax present");
        assert_eq!(se.base, dec!(55410.00)); // round_cents(60,000 × 0.9235)
        assert_eq!(se.ss, dec!(0.00)); // SS cap consumed by the taxpayer's own 168,600 box3
        assert_eq!(se.medicare, dec!(1606.89)); // 2.9% × 55,410
        assert_eq!(ar.se_tax_sch2_l4, dec!(1606.89)); // Sch 2 L4 = ss + medicare (0.9% unbundled)
                                                      // Form 8959: Part I = 0.9% × (Σbox5 280,000 − MFJ 250,000) = 270.00; Part II = se.addl = 498.69.
        assert_eq!(ar.additional_medicare.part1_wages, dec!(270.00));
        assert_eq!(ar.additional_medicare.part2_se, dec!(498.69));
        assert_eq!(ar.additional_medicare.additional_medicare_tax, dec!(768.69)); // L18 → Sch 2 L11
                                                                                  // Part V: L22 = max(0, Σbox6 4,240 − 1.45% × 280,000 (=4,060)) = 180.00 → 1040 25c.
        assert_eq!(ar.additional_medicare.part5_withholding, dec!(180.00));
    }

    // ── Reduce-to-delta: the absolute Form 8960 vs the frozen engine's crypto-delta NIIT (SPEC §5 tail) ──

    /// KAT-5 — with all non-crypto inputs 0, the absolute Form 8960 collapses EXACTLY to the frozen
    /// engine's crypto-delta NIIT in an **NII-binding** regime. Fixture: $250k hobby mining reward (raises
    /// AGI/MAGI but is NOT investment income) + $10k non-business lending interest (the only NII). MAGI
    /// $260k ≫ NII $10k → NII binds; absolute NIIT = 3.8% × 10,000 = $380 = the delta.
    #[test]
    fn kat5_absolute_niit_reduces_to_delta_nii_binding() {
        let ri = single();
        let st = state_income(vec![
            income(IncomeKind::Reward, false, dec!(250000)), // hobby → AGI but not NII
            income(IncomeKind::Interest, false, dec!(10000)), // non-business lending → NII
        ]);
        let params = ty2024_params();
        let table = synthetic_table(2024);
        let ar = assemble_absolute(&ri, &st, &params, &table, 2024);
        assert_eq!(ar.niit.nii, dec!(10000)); // only the lending interest
        assert_eq!(ar.niit.tax, dec!(380.00)); // 3.8% × 10,000 (NII-binding)
                                               // The frozen crypto-delta NIIT on the SAME ledger + a zeroed profile — collapses to the cent.
        let profile = derive_tax_profile(&ri, &params, 2024);
        match compute_tax_year(&[], &st, 2024, Some(&profile), &tables_2024()) {
            TaxOutcome::Computed(r) => assert_eq!(r.niit, ar.niit.tax),
            other => panic!("must compute, got {other:?}"),
        }
    }

    /// The medical-floor channel (SPEC §6 / `p3-crypto-donation-delta-integration`): the ABSOLUTE
    /// Schedule A applies the 7.5% medical floor on the **with-crypto AGI** (G7), so crypto income shrinks
    /// the medical deduction — the one anti-conservative direction the §6 dual report documents (the derive
    /// side fixes the floor at non-crypto AGI, so `absolute_with − absolute_without ≠ delta`).
    #[test]
    fn medical_floor_uses_with_crypto_agi_shrinking_the_deduction() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            schedule_a: Some(ScheduleAInputs {
                medical: dec!(20000),
                mortgage_interest_1098: dec!(30000),
                ..Default::default()
            }),
            ..Default::default()
        };
        let params = ty2024_params();
        let table = real_2024_table();
        // No crypto: AGI 100,000 → floor 7,500 → medical 12,500; itemized 12,500 + 30,000 = 42,500.
        let no_crypto = assemble_absolute(&ri, &empty_ledger(), &params, &table, 2024);
        assert_eq!(no_crypto.itemized_deduction, Some(dec!(42500)));
        // $50k hobby crypto → AGI 150,000 → floor 11,250 → medical 8,750; itemized 8,750 + 30,000 = 38,750.
        let st = state_income(vec![income(IncomeKind::Reward, false, dec!(50000))]);
        let with_crypto = assemble_absolute(&ri, &st, &params, &table, 2024);
        assert_eq!(with_crypto.itemized_deduction, Some(dec!(38750)));
        // The deduction shrank by exactly 7.5% × 50,000 = 3,750 (the with-crypto floor).
        assert_eq!(
            no_crypto.itemized_deduction.unwrap() - with_crypto.itemized_deduction.unwrap(),
            dec!(3750)
        );
    }

    /// A `BTreeMap` tables double carrying the real TY2024 Single+MFJ table (for the frozen delta engine).
    fn tables_real_2024() -> BTreeMap<i32, TaxTable> {
        let mut m = BTreeMap::new();
        m.insert(2024, real_2024_table());
        m
    }

    /// I3 (Fable r1) / §6 — the **medical-floor** divergence: `absolute_with − absolute_without ≠ delta`, and
    /// specifically the delta UNDERSTATES (the one anti-conservative channel). The absolute Schedule A uses
    /// the with-crypto AGI for the 7.5% floor (shrinking the medical deduction), but the frozen delta's
    /// deduction is fixed at the lower non-crypto AGI floor — so it misses the tax on the shrunk deduction.
    #[test]
    fn section6_medical_floor_delta_understates_and_does_not_reconcile() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            schedule_a: Some(ScheduleAInputs {
                medical: dec!(20000),
                mortgage_interest_1098: dec!(30000),
                ..Default::default()
            }),
            ..Default::default()
        };
        let params = ty2024_params();
        let table = real_2024_table();
        let st = state_income(vec![income(IncomeKind::Reward, false, dec!(50000))]);
        let with = assemble_absolute(&ri, &st, &params, &table, 2024).total_tax;
        let without = assemble_absolute(&ri, &empty_ledger(), &params, &table, 2024).total_tax;
        let delta = match compute_tax_year(
            &[],
            &st,
            2024,
            Some(&derive_tax_profile(&ri, &params, 2024)),
            &tables_real_2024(),
        ) {
            TaxOutcome::Computed(r) => r.total_federal_tax_attributable,
            other => panic!("must compute, got {other:?}"),
        };
        assert!(with > without); // crypto adds tax
        assert_ne!(with - without, delta); // §6: the two questions do NOT reconcile
        assert!(with - without > delta); // the delta understates (the medical-floor anti-conservative channel)
    }

    /// I3 (Fable r1) / `p2-pref-over-ti-clamp` — the **pref-over-TI** divergence: the derive-side strip
    /// floors the ordinary base to 0 while the frozen engine stacks the FULL qualified-dividend slice with
    /// no TI cap, so the delta OVERSTATES; the absolute L16 (qdcgt's `min(L1, qd+ltcg)` cap) is correct.
    /// Non-crypto profile has TI < qualified dividends (a retiree shape); adding $5k crypto ordinary income
    /// pushes the frozen engine's uncapped pref across the 0%→15% LTCG breakpoint, but the capped absolute
    /// TI stays in the 0% bracket → absolute crypto tax = $0, delta = $1,250.
    #[test]
    fn section6_pref_over_ti_delta_overstates_and_does_not_reconcile() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(5000), dec!(5000), dec!(5000))],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(50000),
                box1b_qualified: dec!(50000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let params = ty2024_params();
        let table = real_2024_table();
        let st = state_income(vec![income(IncomeKind::Reward, false, dec!(5000))]);
        let with = assemble_absolute(&ri, &st, &params, &table, 2024).total_tax;
        let without = assemble_absolute(&ri, &empty_ledger(), &params, &table, 2024).total_tax;
        // Both absolute totals are $0 — the capped pref (min(TI, qd)) stays entirely in the 0% LTCG bracket.
        assert_eq!(with, Usd::ZERO);
        assert_eq!(without, Usd::ZERO);
        let delta = match compute_tax_year(
            &[],
            &st,
            2024,
            Some(&derive_tax_profile(&ri, &params, 2024)),
            &tables_real_2024(),
        ) {
            TaxOutcome::Computed(r) => r.total_federal_tax_attributable,
            other => panic!("must compute, got {other:?}"),
        };
        assert_eq!(delta, dec!(1250.00)); // the frozen engine's UNCAPPED stacking crosses into 15%
        assert_ne!(with - without, delta); // §6: do not reconcile
        assert!(delta > with - without); // the delta OVERSTATES (the pref-over-TI channel)
    }

    /// KAT-5b — the documented `absolute NIIT < delta` inequality in a **MAGI-binding SE** regime. Fixture:
    /// $210k business mining (Schedule C → SE) + $10k lending. The absolute MAGI is NET of the ½-SE
    /// deduction (which the frozen engine's gross `crypto_ord` cannot see), so the absolute MAGI arm binds
    /// BELOW the frozen NII arm — the §6 divergence: the absolute Form 8960 is the correct filed figure;
    /// the crypto delta is the (over-stated here) attribution. Neither is a bug.
    #[test]
    fn kat5b_absolute_niit_below_delta_magi_binding_se() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![
            mining(dec!(210000)), // business SE income (gross → engine crypto_ord)
            income(IncomeKind::Interest, false, dec!(10000)), // NII
        ]);
        let params = ty2024_params();
        let table = synthetic_table(2024);
        let ar = assemble_absolute(&ri, &st, &params, &table, 2024);
        assert_eq!(ar.niit.nii, dec!(10000));
        // MAGI-binding: absolute NIIT is strictly below 3.8% × NII (the ½-SE deduction shrank the MAGI arm).
        assert!(ar.niit.tax < dec!(380.00));
        assert_eq!(ar.niit.tax, dec!(238.25)); // 3.8% × (206,269.74 − 200,000)
                                               // The frozen delta uses the GROSS crypto AGI (no ½-SE) → its NII arm binds → strictly higher.
        let profile = derive_tax_profile(&ri, &params, 2024);
        match compute_tax_year(&[], &st, 2024, Some(&profile), &tables_2024()) {
            TaxOutcome::Computed(r) => {
                assert_eq!(r.niit, dec!(380.00));
                assert!(ar.niit.tax < r.niit); // documented §6 divergence
            }
            other => panic!("must compute, got {other:?}"),
        }
    }

    // ── §4 R3-M6 carryover write-back (P4.9) ─────────────────────────────────────────────────────
    use crate::tax::return_inputs::{CarryProvenance, QbiInputs};

    /// A fixture whose absolute return has BOTH a nonzero charitable carryover-out (crypto donation over
    /// the 30% ceiling) AND a QBI REIT/PTP loss carryforward-out (prior loss > this year's REIT income).
    fn ar_with_carryovers() -> AbsoluteReturn {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(4000),
                box5_section_199a: dec!(4000), // REIT dividends < the prior loss carryforward
                ..Default::default()
            }],
            qbi: QbiInputs {
                reit_ptp_carryforward_in: dec!(10000),
                ..Default::default()
            },
            ..Default::default()
        };
        let st = state_removals(vec![donation(
            date!(2024 - 06 - 01),
            vec![donation_leg(Term::LongTerm, dec!(20000), dec!(70000))],
        )]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        assert!(!ar.charitable_carryover_out.is_empty()); // there IS a charitable carryover
        assert_eq!(ar.qbi_reit_ptp_carryforward_out, dec!(6000)); // 10,000 prior − 4,000 REIT
        ar
    }

    /// Write-back into a FRESH next year: the computed carryovers become next year's carryover-in, stamped
    /// `Computed` (so a subsequent report can overwrite them silently).
    #[test]
    fn writeback_into_fresh_next_year() {
        let ar = ar_with_carryovers();
        let next = apply_carryover_writeback(&ar, ReturnInputs::default(), false).unwrap();
        assert_eq!(
            next.charitable_carryover_in.len(),
            ar.charitable_carryover_out.len()
        );
        assert_eq!(
            next.charitable_carryover_in[0].amount,
            ar.charitable_carryover_out[0].amount
        );
        assert!(next
            .charitable_carryover_in
            .iter()
            .all(|c| c.provenance == CarryProvenance::Computed));
        assert_eq!(next.qbi.reit_ptp_carryforward_in, dec!(6000));
        assert_eq!(
            next.qbi.reit_ptp_carryforward_in_provenance,
            CarryProvenance::Computed
        );
    }

    /// R3-M6 precedence: a prior COMPUTED carryover-in is overwritten silently (no `--force`).
    #[test]
    fn writeback_overwrites_computed_silently() {
        let ar = ar_with_carryovers();
        let prior = ReturnInputs {
            charitable_carryover_in: vec![CharitableCarryItem {
                class: CharitableClass::Cash60,
                amount: dec!(999),
                origin_year: 2023,
                provenance: CarryProvenance::Computed,
            }],
            qbi: QbiInputs {
                reit_ptp_carryforward_in: dec!(999),
                reit_ptp_carryforward_in_provenance: CarryProvenance::Computed,
            },
            ..Default::default()
        };
        let next = apply_carryover_writeback(&ar, prior, false).unwrap();
        assert_eq!(
            next.charitable_carryover_in[0].amount,
            ar.charitable_carryover_out[0].amount
        );
        assert_eq!(next.qbi.reit_ptp_carryforward_in, dec!(6000));
    }

    /// R3-M6 precedence: a USER-entered carryover-in refuses without `--force`; `--force` overwrites. Both
    /// the charitable and the QBI conflicts are checked BEFORE either field is written (atomic).
    #[test]
    fn writeback_refuses_user_without_force() {
        let ar = ar_with_carryovers();
        // User charitable carryover present → refuse without force.
        let user_charitable = ReturnInputs {
            charitable_carryover_in: vec![CharitableCarryItem {
                class: CharitableClass::Cash60,
                amount: dec!(5000),
                origin_year: 2023,
                provenance: CarryProvenance::User,
            }],
            ..Default::default()
        };
        assert!(apply_carryover_writeback(&ar, user_charitable.clone(), false).is_err());
        assert!(apply_carryover_writeback(&ar, user_charitable, true).is_ok()); // --force overwrites
                                                                                // User QBI carryforward present → refuse without force (atomic: charitable not half-written).
        let user_qbi = ReturnInputs {
            qbi: QbiInputs {
                reit_ptp_carryforward_in: dec!(3000),
                reit_ptp_carryforward_in_provenance: CarryProvenance::User,
            },
            ..Default::default()
        };
        assert!(apply_carryover_writeback(&ar, user_qbi.clone(), false).is_err());
        assert!(apply_carryover_writeback(&ar, user_qbi, true).is_ok());
    }

    /// M3 (Fable P4.9 r1): serde back-compat — a LEGACY blob with no `provenance` key loads as `User`, so a
    /// pre-existing (imported) carryover is protected from a silent write-back overwrite.
    #[test]
    fn legacy_carryover_blob_without_provenance_loads_as_user() {
        let json = r#"{"filing_status":"Single",
            "charitable_carryover_in":[{"class":"cash60","amount":"5000","origin_year":2023}],
            "qbi":{"reit_ptp_carryforward_in":"2000"}}"#;
        let ri: ReturnInputs = serde_json::from_str(json).unwrap();
        assert_eq!(
            ri.charitable_carryover_in[0].provenance,
            CarryProvenance::User
        );
        assert_eq!(
            ri.qbi.reit_ptp_carryforward_in_provenance,
            CarryProvenance::User
        );
        // …and is therefore protected: the write-back refuses without --force.
        let ar = ar_with_carryovers();
        assert!(apply_carryover_writeback(&ar, ri.clone(), false).is_err());
        assert!(apply_carryover_writeback(&ar, ri, true).is_ok());
    }
}
