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
use crate::tax::capital_loss_carryover::{
    CapitalLossCarryoverInputs, CapitalLossCarryoverWorksheet,
};
use crate::tax::charitable::apply_170b;
use crate::tax::compute::{net_1222, CapNet};
use crate::tax::method::qdcgt_line16;
use crate::tax::other_taxes::{form_8959, form_8960, sch2_line4_se, Form8959, Form8960};
use crate::tax::qbi::{compute_8995, has_qbi};
use crate::tax::qbi_a::{qbi_after_sstb_exclusion, Qbi199aRegime};
use crate::tax::return_inputs::{
    CarryProvenance, CharitableCarryItem, CharitableClass, CharitableGift, Owner, ReturnInputs,
};
use crate::tax::return_refuse::{Refusal, RefuseReason};
use crate::tax::se::{compute_se_tax, se_net_income, SeTaxResult};
use crate::tax::tables::{loss_limit, FullReturnParams, TaxTable, EMPLOYEE_OASDI_RATE};
use crate::tax::types::{Carryforward, FilingStatus, TaxProfile};
use crate::IncomeKind;
use rust_decimal_macros::dec;
use time::{Date, Month};

// ── §63 standard deduction (Phase 3 task 1) ──────────────────────────────────────────────────────

/// Whether `dob` makes a person **aged (65+)** for the §63(f) additional standard deduction in tax `year`.
/// IRS rule (Pub 501): 65 if born **on or before January 1 of `year − 64`** (turned 65 by the Jan-1-after-
/// year-end test). A `None` DOB is "not established" → NOT counted as aged: the conservative, fail-closed
/// direction — never grant an unsubstantiated deduction, and never silently assume a birthdate
/// (burns down the `dob-option-pin` follow-up; §4.2 / review r1-M3).
///
/// ★★ **AND the death carve-out (`FOLLOWUPS.md` §G-9 — a live defect through v0.14.0).** i1040gi:
/// *"If your spouse was born before January 2, 1960, but died in 2024 before reaching age 65, don't
/// check the box … A person is considered to reach age 65 on the day before the person's 65th
/// birthday."* This function decided the box from the date of BIRTH alone, so a spouse who died
/// in-year before turning 65 was granted a $1,550 addition they were not entitled to — understating
/// tax, and invisible to both oracles (OTS takes a filer-answered Y/N; taxcalc has only `age_spouse`).
pub(crate) fn is_aged(
    dob: Option<Date>,
    died_during_year: Option<bool>,
    date_of_death: Option<Date>,
    year: i32,
) -> bool {
    let Some(d) = dob else {
        return false;
    };
    if !born_early_enough(d, year) {
        return false;
    }
    match (died_during_year, date_of_death) {
        // Answered "did not die during the tax year" — the date-of-birth test is the whole test, which
        // is what this function did unconditionally before §G-9.
        (Some(false), _) => true,
        // Died in-year with the date on file: reached 65 on or before that day?
        (_, Some(dod)) => reaches_65_on(d).is_some_and(|reached| dod >= reached),
        // Died in-year, date SKIPPED. Class (B): the burden to claim the addition is the filer's (New
        // Colonial Ice), so silence is lawful and forgoes it. Never granted on an unknown date.
        (Some(true), None) => false,
        // ★★ UNANSWERED — and since the death questions became SKIPPABLE this arm is REACHABLE on an
        // ordinary emitting path, not a defensive backstop. It used to be unreachable because
        // `screen_inputs` refused first; that refusal is gone, precisely BECAUSE this arm already
        // fails in the safe direction. Silence forgoes the addition (tax OVERSTATED), never grants it
        // on an unverified birthdate — which is the whole of §G-9's fix. `Advisory::
        // AgedBoxForfeitedDeathUnanswered` is what tells the filer it cost them something.
        // **Do not "simplify" this to `true`.** That is the shipped v0.14.0 defect, restored.
        (None, None) => false,
    }
}

/// Born on or before January 1 of `year − 64` — i.e. aged 65 or over by the §63(f) test, ignoring the
/// death carve-out. Shared by [`is_aged`] and the forfeited-box advisory so the cutoff has ONE
/// definition: an advisory that fired on a different cutoff than the deduction would be worse than no
/// advisory, because it would name a benefit the return was never going to grant.
pub(crate) fn born_early_enough(dob: Date, year: i32) -> bool {
    Date::from_calendar_date(year - 64, Month::January, 1).is_ok_and(|cutoff| dob <= cutoff)
}

/// Would this person's §63(f) aged box have been checked, but for their death question being
/// UNANSWERED? True exactly when the skip costs the filer the addition: a qualifying date of birth is
/// on file, no date of death is on file, and the gate is `None`.
///
/// Drives [`crate::tax::advisories::Advisory::AgedBoxForfeitedDeathUnanswered`]. Deliberately narrow —
/// an advisory that fires when nothing was forgone trains the filer to ignore the advisory list.
pub(crate) fn aged_box_forgone_for_unanswered_death(
    dob: Option<Date>,
    died_during_year: Option<bool>,
    date_of_death: Option<Date>,
    year: i32,
) -> bool {
    dob.is_some_and(|d| born_early_enough(d, year))
        && died_during_year.is_none()
        && date_of_death.is_none()
}

/// The day a person born on `dob` **reaches 65** — the day *before* the 65th birthday, per the IRS's
/// own sentence. `None` only if the date arithmetic is impossible.
///
/// ★ The leap-day case is why this is its own function: someone born **February 29** has no 65th
/// birthday, and the convention is that they attain the age on **March 1**, so they reach 65 on
/// February 28. Computing `birthday − 1 day` without handling that panics or silently shifts a year.
fn reaches_65_on(dob: Date) -> Option<Date> {
    let sixty_fifth = dob.replace_year(dob.year() + 65).ok().or_else(|| {
        // Feb 29 → the 65th "birthday" is taken as Mar 1 of that year.
        Date::from_calendar_date(dob.year() + 65, Month::March, 1).ok()
    })?;
    sixty_fifth.previous_day()
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
/// §63(f) boxes: the taxpayer always, plus the spouse whenever `questions::spouse_63f_boxes_count`
/// says so — MFJ always, and MFS when i1040gi's three conditions (*"no income, isn't filing a return,
/// can't be claimed as a dependent"*) are ALL affirmatively answered. The gate fails closed: any
/// unanswered or adverse condition forgoes the box, never over-granting.
pub fn standard_deduction(
    ri: &ReturnInputs,
    params: &FullReturnParams,
    year: i32,
    dependent_earned_income: Usd,
) -> Usd {
    let status = ri.filing_status;
    let basic = params.std_deduction_for(status);
    // ★ `!= Some(false)` — i.e. UNKNOWN takes the dependent branch, not the basic-std branch (D-8).
    //
    // `screen_inputs` refuses an unanswered flag, so `None` cannot reach here on any real path. This is
    // defense-in-depth, and what matters is its DIRECTION: the §63(c)(5) floor is never larger than the
    // basic std, so an unknown flag can only OVERSTATE tax. `unwrap_or(false)` — the idiom that caused
    // this defect — points the other way and silently understates. Both are one token; only one is safe.
    let base = if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false) {
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

/// The INCOME-tax path of Schedule A line 5a: W-2 state/local withholding (box 17/19) + state estimated
/// payments + a prior-year balance paid. ★ ONE derivation (Fable IMPL r3 MINOR-1): both `salt_line_5a`
/// (when the §164(b)(5) election is off) and the `SalesTaxElectionWithoutAmount` refusal (which must know
/// whether electing sales tax would COLLAPSE this to $0) call it, so the deducted set and the guarded set
/// cannot drift — the "one derivation" rule §2.7 states.
pub fn income_tax_salt(ri: &ReturnInputs, a: &crate::tax::return_inputs::ScheduleAInputs) -> Usd {
    let w2_wh: Usd = ri
        .w2s
        .iter()
        .map(|w| w.box17_state_tax_withheld + w.box19_local_tax)
        .sum();
    w2_wh + a.salt_state_estimated_payments + a.salt_prior_year_balance_paid
}

/// The §164(b)(5) SALT line 5a election: `true` (sales-tax path) → `salt_sales_tax_amount` ONLY; `false`
/// (income-tax path) → [`income_tax_salt`]. (A nonzero `salt_sales_tax_amount` with the election OFF is
/// refused upstream — R3-M9.)
fn salt_line_5a(ri: &ReturnInputs, a: &crate::tax::return_inputs::ScheduleAInputs) -> Usd {
    if a.salt_use_sales_tax == Some(true) {
        a.salt_sales_tax_amount
    } else {
        income_tax_salt(ri, a)
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
    /// **L1a (d)** — *"Totals for all short-term transactions reported on Form 1099-B for which basis
    /// was reported to the IRS and for which you have no adjustments"*, proceeds. §G-28/B4.
    ///
    /// ★ Distinct from line 3, which is the attached Form 8949's Box C/I total. These transactions are
    /// NOT on the 8949 at all — that is the whole point of line 1a, and why btctax needs no second
    /// lot-level engine to report them.
    pub st_1099b_proceeds_1ad: Usd,
    /// **L1a (e)** — the same transactions' cost basis.
    pub st_1099b_cost_1ae: Usd,
    /// **L1a (h)** — *"Subtract column (e) from column (d)…"*, signed.
    pub st_1099b_gain_1ah: Usd,
    /// **L8a (d)** — the long-term counterpart of line 1a. §G-28/B4.
    pub lt_1099b_proceeds_8ad: Usd,
    /// **L8a (e)** — cost basis.
    pub lt_1099b_cost_8ae: Usd,
    /// **L8a (h)** — signed gain or loss.
    pub lt_1099b_gain_8ah: Usd,
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
    /// §164(b)(5): the filer ELECTED sales taxes instead of income taxes on line 5a. Core already honours
    /// this in the arithmetic; Schedule A's own 5a CHECKBOX must say so, or the filed form claims income
    /// taxes it did not use (Q7 item 3).
    pub salt_is_sales_tax: bool,
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
    /// The APPLIED SALT limit — i.e. **line 5e itself** (`salt_cap = salt_5e`). Carried because the
    /// PRINTED line 5e must cap the PRINTED line 5d, and the printed chain cannot re-derive the limit
    /// without doing tax logic in the forms crate, which is precisely what it must not do. The
    /// recomputation `min(5d, salt_cap)` is then idempotent, since 5e ≤ 5d always.
    ///
    /// ★★★ **IT IS NOT §164(b)(6)'s $10,000/$5,000-MFS CEILING**, and this doc used to say it was
    /// (final whole-branch review, P3-2). Whenever 5d is under the statutory ceiling, `salt_cap` is
    /// simply 5d — the filer's own SALT total. The false name leaked into a filer-facing refusal,
    /// which told a filer with 5a = $4,000 and 5b = $3,000 that "§164(b)(6)'s cap" was **$7,000**.
    /// The statute's cap is $10,000; $7,000 was their own line 5e.
    pub salt_cap: Usd,
    /// L8a — home-mortgage interest reported on Form 1098. ★ §2.7: **$0** when [`mortgage_mixed_use_box`]
    /// is set — a mixed-use mortgage's non-acquisition portion is non-deductible (§163(h)(3)(F)) and v1
    /// cannot compute the Pub. 936 split, so it claims none of it.
    pub mortgage_8a: Usd,
    /// ★ §2.7 — Schedule A **line 8's checkbox**: "If you didn't use all of your home mortgage loan(s) to
    /// buy, build, or improve your home, check this box." Set iff the filer declared a mixed-use mortgage
    /// (`mortgage_all_used_to_buy_build_improve == Some(false)`) on a Schedule A that reports 1098 interest —
    /// see [`mixed_use_mortgage_forgone`], the single derivation this and [`mortgage_8a`] share.
    pub mortgage_mixed_use_box: bool,
    /// **L9 — "Investment interest. Attach Form 4952 if required."** (§163(d)). Taken at the amount
    /// the filer entered, which is sound ONLY because `RefuseReason::Form4952Required` stands in
    /// front of it: it refuses when Form 4952 is being filed, and when the amount breaks i4952's
    /// no-filing exception (the §163(d)(1) net-investment-income limit btctax does not compute).
    pub investment_interest_9: Usd,
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
/// ★ §2.7 — the ONE derivation of the §163(h)(3)(F) mixed-use-mortgage disposition, shared by
/// [`schedule_a_parts`] (which zeroes line 8a and checks the line-8 box) and
/// [`crate::tax::advisories`] (the owner-mandate note). Returns the CEILING of interest a Pub. 936
/// allocation could restore — the full `mortgage_interest_1098`, documented as "up to" — when the
/// mixed-use question is LIVE (a `schedule_a` reporting 1098 interest > 0) and the filer answered
/// `Some(false)`; `None` otherwise. v1 cannot compute the allocation, so it deducts none of it
/// ($0 ≤ the true allocation, always ⇒ tax overstated, never understated).
/// ★★★ **Form 8960 line 9b's §164(b)(6) BOUND** — the most state/local income tax this return could
/// possibly allocate to net investment income. The ONE derivation, shared by the `screen_absolute`
/// refusal that enforces it and the advisory that names it, so the gate and the note cannot disagree
/// about the number (the r3 MINOR-1 lesson).
///
/// §1411(c)(1)(B) reduces net investment income only by *"the deductions **allowed by this
/// subtitle**"*, and §164(b)(6)(B) caps the aggregate SALT *"taken into account"* at $10,000 ($5,000
/// MFS) — so tax paid above the cap is not a deduction at all and there is nothing to allocate
/// (`ADJUDICATION-2026-08-21.md` D5). i8960's allocation block names the same quantity from the other
/// side: *"State, local, and foreign income taxes **if properly deducted on your return** when
/// calculating your U.S. regular income tax."*
///
/// The three branches, each of which independently zeroes it:
///
/// - **no Schedule A / the standard deduction won** ⇒ `$0`. Nothing was deducted on the return, so
///   nothing is allocable. This is the plain reading of "properly deducted on your return", and it is
///   why the bound reads `deduction_is_itemized` (the computed §63(e) election) rather than merely
///   `schedule_a.is_some()`.
/// - **the §164(b)(5) general-sales-tax election** ⇒ `$0`. i8960, Line 9b: *"Sales taxes aren't
///   deductible in computing net investment income."* Line 5a is then sales tax, and 5e is a total
///   that includes it — so a bound built from 5e alone would launder sales tax into a §1411 deduction.
/// - otherwise `min(line 5a, line 5e, salt_cap)`. 5a is the income-tax component and 5e is what
///   survived the §164(b)(6) limit, so the bound is the smaller of those two.
///
///   ★ **The third term is redundant BY CONSTRUCTION, not "while 5e ≤ cap"** (final whole-branch
///   review, P3-2). `ScheduleAParts::salt_cap` is assigned `salt_5e` — it is the APPLIED limit, not
///   §164(b)(6)'s $10,000/$5,000-MFS ceiling — so `.min(salt_cap)` is `.min(salt_5e)` and can never
///   bind. It is kept because it is harmless and the printed chain reads the same field, but the doc
///   that called it "the statute's own ceiling, transcribed anyway" was describing a term that is not
///   there. The bound's VALUE was and is correct in every case; only this claim about it was wrong.
///
/// ★ **Foreign income tax contributes nothing**, and not by omission: btctax's only foreign income
/// tax (1099-INT box 6 / 1099-DIV box 7) is taken unconditionally as the §904(j) CREDIT, and
/// §275(a)(4) — which i8960 cites on this very line — then denies the deduction.
///
/// ★★ It is a BOUND, never a value. btctax does not choose the filer's "reasonable method"; this only
/// says how large the pool that method divides can be.
pub fn nii_line9b_bound(ar: &AbsoluteReturn) -> Usd {
    if !ar.deduction_is_itemized {
        return Usd::ZERO;
    }
    let Some(a) = ar.schedule_a.as_ref() else {
        return Usd::ZERO;
    };
    if a.salt_is_sales_tax {
        return Usd::ZERO;
    }
    a.salt_5a.min(a.salt_5e).min(a.salt_cap)
}

pub fn mixed_use_mortgage_forgone(ri: &ReturnInputs) -> Option<Usd> {
    let a = ri.schedule_a.as_ref()?;
    (a.mortgage_all_used_to_buy_build_improve == Some(false)
        && a.mortgage_interest_1098 > Usd::ZERO)
        .then_some(a.mortgage_interest_1098)
}

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
    // ★ ONE call, and the printed form reads the RESULT (`ScheduleAParts::salt_cap`), so `printed.rs`
    //   never learns which year's instrument ran. §164(b)(7)(B)(iv) modified AGI is AGI plus the
    //   §911/931/933 exclusions the filer declared; `None` means the exclusion gate was never
    //   answered, which `screen_inputs` refuses and `line_5e` fails closed on.
    let salt_5e = params
        .salt
        .line_5e(salt_5d, ri.modified_agi(agi), ri.filing_status);
    // The APPLIED limit carried to the printed form. `printed.rs` recomputes `min(5d, salt_cap)`,
    // which is idempotent once this holds the worksheet's own line 10 (5e ≤ 5d always).
    let salt_cap = salt_5e;

    // Line 8a — home-mortgage interest (points/8b are refuse-or-advise). ★ §2.7: a MIXED-USE mortgage
    // (`Some(false)`) zeroes 8a and checks the line-8 box — v1 cannot do the Pub. 936 split, so it deducts
    // none of the interest. Both key on the SINGLE `mixed_use_mortgage_forgone` derivation.
    let mortgage_mixed_use_box = mixed_use_mortgage_forgone(ri).is_some();
    let mortgage_8a = if mortgage_mixed_use_box {
        Usd::ZERO
    } else {
        a.mortgage_interest_1098
    };

    Some(ScheduleAParts {
        salt_is_sales_tax: a.salt_use_sales_tax == Some(true),
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
        mortgage_mixed_use_box,
        investment_interest_9: a.investment_interest,
        charitable_cash_11: charitable.allowed_cash,
        charitable_noncash_12: charitable.allowed_noncash,
        charitable_carryover_13: charitable.allowed_carryover,
        charitable_14: charitable.allowed,
        // L17 = 4 + 7 + 10 + 14, and line 10 is "add 8e and 9" — so line 9 belongs in the total.
        total_17: medical_allowed
            + salt_5e
            + mortgage_8a
            + a.investment_interest
            + charitable.allowed,
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
/// §G-28/B4 — the Form 1099-B totals for Schedule D lines **1a** and **8a**, as `(short-term gain,
/// long-term gain)`. Each is `Σ(proceeds − basis)` over the filer's 1099-B rows, and each may be
/// NEGATIVE: a broker's netted totals are a loss as readily as a gain, and §1222 nets within character
/// before the §1211 limit applies.
///
/// ★★ Summed as GAINS per row, not as (Σproceeds − Σbasis), because that is what the printed column
/// (h) is per row — and the two agree only while every row is present. Keeping the per-row shape means
/// a future per-row output (a second Schedule D page, say) needs no re-derivation.
pub fn form_1099b_gains(ri: &ReturnInputs) -> (Usd, Usd) {
    ri.b_1099
        .iter()
        .fold((Usd::ZERO, Usd::ZERO), |(st, lt), b| {
            (
                st + (b.short_term_proceeds - b.short_term_basis),
                lt + (b.long_term_proceeds - b.long_term_basis),
            )
        })
}

/// ★ N1 — `pub(crate)` so [`crate::tax::advisories`] can quote the FLAT `st_carry`/`lt_carry` the
/// frozen delta engine prints beside the worksheet's answer, rather than re-deriving the flat rule and
/// creating a second authority for it.
pub(crate) fn capital_net(
    ri: &ReturnInputs,
    state: &LedgerState,
    year: i32,
    status: FilingStatus,
) -> CapNet {
    let sd = schedule_d(state, year); // raw crypto ST/LT nets (traverses state.disposals)
    let cf = ri.capital_loss_carryforward_in;
    // ★★★ §G-28/B4 — the broker totals join the crypto nets HERE, at the §1222 within-character
    //     netting, which is where Schedule D combines lines 1a..6 and 8a..14. `net_1222` itself is in
    //     the FROZEN delta engine (`frozen_guard.rs`), so only its ARGUMENTS change — the securities
    //     gain is added to the same character it belongs to and nothing inside is touched.
    let (b_st, b_lt) = form_1099b_gains(ri);
    net_1222(
        sd.st.gain + b_st,
        sd.lt.gain + b_lt,
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

/// An EIN reduced to its nine digits, or `None` if it is absent, blank, or not nine digits.
///
/// ★ Modelled on [`crate::tax::packet::Ssn::canonical`] — strip hyphens and whitespace, then require
///   exactly nine digits. Identity comparisons must never be spelling comparisons: `11-1111111` and
///   `111111111` are ONE employer, and treating them as two understates tax (§6413(c)).
///
/// Returns `None` rather than an error because the caller already distinguishes "cannot decide" from
/// "decided": a malformed or missing EIN reaches the screen, which refuses and asks.
pub(crate) fn canonical_ein(raw: &str) -> Option<String> {
    let digits: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    (digits.len() == 9 && digits.chars().all(|c| c.is_ascii_digit())).then_some(digits)
}

/// One employer's over-cap Social Security withholding for one person — real money, recoverable, and
/// **not claimable on this return**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonCreditableSs {
    /// Whose W-2 it was. The §3101(a) cap is per person, and so is the remedy.
    pub owner: Owner,
    /// The employer to ask, canonicalized to nine digits.
    pub ein: String,
    /// How much that employer withheld above the cap.
    pub amount: Usd,
}

/// Every (person, employer) pair whose withholding exceeded the §3101(a) cap — the amounts §6413(c)
/// will not credit, itemized so the filer knows **which employer to ask and for how much**.
///
/// ★★★ PER EMPLOYER, not per person, because that is what i1040gi says: *"But if **any one employer**
/// withheld more than $10,453.20, you can't claim the excess on your return. The employer should adjust
/// the tax for you."* An earlier version fired only when a person had EXACTLY ONE employer, which is a
/// different test and silently dropped the disclosure in the common case. Review r8 showed the sharpest
/// consequence: adding a second employer who withheld **nothing** left the tax outcome byte-identical
/// ($0 credit, the same amount stranded) and switched the disclosure OFF.
///
/// ★★ And it is a LIST, not a scalar. Summing an MFJ couple's two amounts produced one number that no
/// employer withheld, attached to a message telling the filer to "ask that employer" — an employer that
/// does not exist. The credit is figured per person and never pooled; so is this.
pub(crate) fn non_creditable_ss(ri: &ReturnInputs, table: &TaxTable) -> Vec<NonCreditableSs> {
    let max = table.ss_wage_base * EMPLOYEE_OASDI_RATE;
    let mut out = Vec::new();
    for owner in [Owner::Taxpayer, Owner::Spouse] {
        let mine: Vec<_> = ri.w2s.iter().filter(|w| w.owner == owner).collect();
        let mut by_ein: std::collections::BTreeMap<String, Usd> = std::collections::BTreeMap::new();
        for w in &mine {
            if let Some(e) = w.ein.as_deref().and_then(canonical_ein) {
                *by_ein.entry(e).or_insert(Usd::ZERO) += w.box4_ss_withheld;
            }
        }
        for (ein, withheld) in by_ein {
            if withheld > max {
                out.push(NonCreditableSs {
                    owner,
                    ein,
                    amount: withheld - max,
                });
            }
        }
    }
    out
}

/// §6413(c) **excess Social Security** credit (Schedule 3 line 11), PER PERSON/// §6413(c) **excess Social Security** credit (Schedule 3 line 11), PER PERSON — never pooled (§4.9).
///
/// ★★★ Transcribed from i1040gi's two sentences, both of which are conditions:
///
/// > *"If you, or your spouse if filing a joint return, had **more than one employer** for 2024 and
/// > total wages of more than $168,600, too much social security or tier 1 railroad retirement (RRTA)
/// > tax may have been withheld. You can take a credit on this line for the amount withheld in excess
/// > of $10,453.20. But if **any one employer** withheld more than $10,453.20, you can't claim the
/// > excess on your return. The employer should adjust the tax for you."*
///
/// ★★ The previous version enforced only the second, and justified the omission with an equivalence
/// that is false: *"a single-employer person nets 0, so the 'requires ≥ 2 employers' rule falls out
/// naturally."* **One employer may issue several W-2s to one person** — a corrected W-2, a mid-year
/// payroll-system change, separate establishments under one EIN — each under the per-W-2 cap and
/// summing over it. A filing trial credited **$3,894** to a filer entitled to $0, turning an $1,085
/// liability into a $2,809 refund. That is an understatement on a return signed under §6065, the one
/// direction this codebase promises never to go, and it is invisible to both oracles because the credit
/// is a value they are HANDED, not one they derive.
///
/// Returns `Err` when the answer depends on employer identity the filer never supplied — fail loud and
/// collect it, never guess (`CLAUDE.md`: *"If the form asks something our input surface cannot answer,
/// collect it."*).
pub(crate) fn excess_social_security(ri: &ReturnInputs, table: &TaxTable) -> Usd {
    let max = table.ss_wage_base * EMPLOYEE_OASDI_RATE;
    let per_person = |owner: Owner| -> Usd {
        let mine: Vec<&crate::tax::return_inputs::W2> =
            ri.w2s.iter().filter(|w| w.owner == owner).collect();
        let withheld: Usd = mine.iter().map(|w| w.box4_ss_withheld).sum();
        // No excess at all ⇒ no credit, and employer identity never matters. This is the overwhelming
        // common case and must not demand an EIN nobody needed.
        if withheld <= max {
            return Usd::ZERO;
        }
        // Over the cap ⇒ the credit turns on employer identity, so it must be stated.
        // ★★★ CANONICALIZED, NOT TRIMMED. An EIN has two standard renderings — `11-1111111` off the
        //     paper W-2 and `111111111` off a payroll-portal export — and to a string compare those are
        //     TWO EMPLOYERS. That restores the exact understatement this function exists to kill: a
        //     Fable review built the probe (one employer, a W-2 and its W-2c, box 4 $6,200 each, the two
        //     spellings) and got a $1,946.80 credit for a filer entitled to $0.
        //
        //     ★★ The field's own doc comment stated the governing rule while the code violated it —
        //     *"two spellings of one employer are two employers to a string compare"* — and §6413(c) was
        //     then decided by a string compare. Same shape as the equivalence comment this whole fix
        //     replaced.
        let mut eins: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for w in &mine {
            match w.ein.as_deref().and_then(canonical_ein) {
                Some(e) => {
                    eins.insert(e);
                }
                // Unreachable in practice — `screen_inputs` refuses this case before any figure is
                // assembled. Zero is the conservative fallback if that screen is ever bypassed: it
                // OVERSTATES tax, which is the only direction this codebase tolerates being wrong in.
                None => return Usd::ZERO,
            }
        }
        // *"had more than one employer"* — the first condition, and the one that was missing.
        if eins.len() < 2 {
            return Usd::ZERO;
        }
        // *"But if any one employer withheld more than $10,453.20, you can't claim the excess"* — so
        // each EMPLOYER contributes at most the cap before the aggregate is compared to it.
        let creditable: Usd = eins
            .iter()
            .map(|e| {
                let per_employer: Usd = mine
                    .iter()
                    .filter(|w| w.ein.as_deref().and_then(canonical_ein).as_ref() == Some(e))
                    .map(|w| w.box4_ss_withheld)
                    .sum();
                per_employer.min(max)
            })
            .sum();
        (creditable - max).max(Usd::ZERO)
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
    // ★ Non-crypto NONCASH gifts, keyed on the TOTAL noncash the return claims (Fable P6 r1 I6). The
    // $500 trigger printed on Schedule A line 12 — and Form 8283's own "…if you claimed a total deduction
    // of over $500 for ALL contributed property" — is an AGGREGATE over every noncash gift. Keying the
    // refusal on the user-entered gifts alone let a MIXED year through: $300 of user noncash (under the
    // threshold) + $400 of crypto donations from the ledger ⇒ L12 = $700, an 8283 IS required, and the one
    // btctax attaches lists only the crypto rows — an incomplete required attachment, and the §170(f)(11)
    // denial risk the guard exists to prevent. This needs the LEDGER, so it screens here, not in
    // `screen_inputs`.
    if let Some(a) = &ri.schedule_a {
        let user_noncash: Usd = a
            .charitable
            .iter()
            .filter(|g| !matches!(g.class, CharitableClass::Cash60 | CharitableClass::Cash30))
            .map(|g| g.amount)
            .sum();
        let crypto_noncash = crate::forms::year_donation_deduction(state, year);
        if user_noncash > Usd::ZERO
            && user_noncash + crypto_noncash > crate::tax::printed::FORM_8283_THRESHOLD
        {
            return Some(Refusal {
                reason: RefuseReason::NonCryptoNoncashGift,
                detail:
                    "a non-crypto NONCASH charitable gift pushes total noncash gifts over $500, which \
                     requires a Form 8283 listing ALL of the contributed property — and btctax holds no \
                     details for property that did not come from your ledger (description, acquisition \
                     date, appraiser). Complete Form 8283 by hand, or remove the gift."
                        .to_string(),
            });
        }
    }

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
            // §G-28/B3 — Schedule C line 1 is the SUM: the ledger's SE-eligible business crypto plus
            // any non-ledger receipts. Testing the loss against the crypto half alone would refuse a
            // filer whose consulting revenue covers their expenses.
            let net = crypto.business_se_gross + sc.other_gross_receipts - sc.expenses;
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
    // `!= Some(false)`: an unknown flag RUNS the kiddie-tax screen rather than skipping it (fail-closed —
    // skipping can only under-refuse). Unreachable past `screen_inputs`; see `standard_deduction`.
    if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false) {
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
                                                 // ★★★ §G-28/B4 — THE BROKER TOTALS ARE NON-CRYPTO CAPITAL GAIN AND BELONG IN THIS PROFILE.
                                                 //
                                                 //     This is the delta engine's BASELINE: `compute_tax_year` runs `net_1222` twice, once with the
                                                 //     crypto legs and once without, and prices the difference. Everything NON-crypto has to be in
                                                 //     the profile or the crypto slice is stacked from the wrong bottom.
                                                 //
                                                 //     ★★ Before B4, `cap_gain_distr` (1099-DIV box 2a) was the ONLY non-crypto capital-gain
                                                 //     channel in `ReturnInputs`, so `other_net_capital_gain: cap_gain_distr` was complete. B4 added
                                                 //     two more and threaded neither — found independently by BOTH review lenses. On a $2,000,000
                                                 //     broker long-term gain plus $100,000 of crypto LTCG the engine reported ≈$7,946 of
                                                 //     crypto-attributable tax against a true ≈$23,800: it stacked the crypto slice from zero
                                                 //     instead of from $2M, so the §1(h) rate came out 15% where it is 20%, and MAGI missed $2M so
                                                 //     the §1411 threshold never tripped. **Understates**, and the optimizer picks a lot method by
                                                 //     minimizing this number.
                                                 //
                                                 //     ★ The LONG-TERM half needs no frozen-file change: `TaxProfile::other_net_capital_gain` is
                                                 //     documented as exactly "non-crypto net LT-character capital gain", and `derive_tax_profile`
                                                 //     is not in the pinned set. The SHORT-TERM half has no profile field and no `net_1222`
                                                 //     argument, so it is §G-30-shaped and filed there.
    let (_b1099_st, b1099_lt) = form_1099b_gains(ri);

    // ★★★ THE NON-CRYPTO CAPITAL RESULT, COMPUTED THE WAY THE ENGINE WILL COMPUTE IT.
    //
    //     `compute_tax_year` runs `without = net_1222(0, 0, other_net_capital_gain, cf.short,
    //     cf.long, limit)` and then `bottom_without = ordinary_taxable_income + without.ordinary_gain
    //     − without.loss_deduction`. So the contract on this profile is exact and in two parts:
    //       · `magi_excluding_crypto` INCLUDES the non-crypto capital result — `compute.rs` says so in
    //         terms: *"already includes QD + non-crypto cap gain"* — and
    //       · `ordinary_taxable_income` EXCLUDES it, because the engine adds it back.
    //     Calling `net_1222` here with the SAME arguments the engine will use makes the two agree by
    //     construction rather than by coincidence.
    //
    //     ★★★ THE FOLD THAT INTRODUCED THIS ADDED THE RAW SIGNED GAINS INSTEAD, and it was two
    //     Criticals. `cap_gain_distr` (1099-DIV box 2a) is a DISTRIBUTION and can never be negative,
    //     so for its whole life `income_total` could add it unlimited and be right. A broker total can
    //     be a LOSS, and adding one raw deducts it from AGI at full size — where §1211(b) caps the
    //     deduction at $3,000 ($1,500 MFS). A $100,000 broker short-term loss moved MAGI from $200,000
    //     to $100,000 where the truth is $197,000: **$6,590.25** of understated crypto-attributable
    //     tax, on the number `optimize` minimizes to pick a lot method.
    //
    //     ★ That figure was quoted as $6,704 when this fix landed, inherited from the review that
    //       found the defect and not recomputed. It is $114 out — the source used NII $50,000 and
    //       omitted Form 8960 line 5a's §1211(b) −$3,000, giving NIIT $1,900 where it is $1,786. A
    //       number carried across from a review is still a number this file asserts.
    //
    //     ★★ THE SHORT-TERM HALF IS DELIBERATELY ABSENT. `net_1222` has no short-term "other" slot and
    //     `TaxProfile` no field for one, so the engine's `without` cannot see a broker short-term
    //     position at all. Putting it in the profile ANYWAY — which the fold did — makes
    //     `magi_excluding_crypto` and the engine's own `without` disagree, which is exactly how both
    //     Criticals happened. Excluding it keeps the two consistent by construction; the residual gap
    //     is §G-30's, recorded there with its direction.
    let noncrypto_cap = crate::tax::compute::net_1222(
        Usd::ZERO,
        Usd::ZERO,
        cap_gain_distr + b1099_lt,
        ri.capital_loss_carryforward_in.short,
        ri.capital_loss_carryforward_in.long,
        loss_limit(status),
    );
    // 1040 line 7 as the NON-crypto return would print it — gains net of the §1211(b)-limited loss.
    let noncrypto_cap_agi = noncrypto_cap.ordinary_gain + noncrypto_cap.preferential_gain
        - noncrypto_cap.loss_deduction;

    // Sch 1 Part I additional income (non-crypto): L1 taxable state refund + L7 Σ unemployment.
    // (L3 Schedule C's CRYPTO half and L8v digital-asset income are crypto → excluded from the frozen
    // profile; the NON-ledger half of Schedule C is not, and is added below.)
    //
    // ★★★ §G-28/B3 — the non-ledger Schedule C revenue is NON-CRYPTO income by definition, so it
    //     belongs in the delta engine's baseline for exactly the reason `wages` does. Omitting it
    //     priced the crypto ordinary slice from a bracket bottom $85,000 too low and could read the
    //     §1411 MAGI test as under-threshold for a filer who is over — **understating**, which is the
    //     opposite of the safe direction §G-30 first claimed for B3.
    //
    //     ★ GROSS receipts, not net, and the reason is the profile's own standing convention rather
    //       than a double-count argument. `TaxProfile::schedule_c_expenses` is consumed by
    //       `compute_se_tax` ONLY; its own doc says so in terms — *"the income-tax stack (engine B /
    //       `crypto_ord`) is NOT adjusted — the ordinary-income overstatement is disclosed via the
    //       render advisory"*. So expenses never reduce this baseline for anyone, and matching that
    //       keeps the non-crypto slice on the same footing as the crypto one. It also errs the SAFE
    //       way: a baseline slightly high stacks the crypto slice higher, OVERSTATING the
    //       crypto-attributable figure rather than understating it.
    let non_ledger_sch_c = ri
        .schedule_c
        .as_ref()
        .map(|c| c.other_gross_receipts)
        .unwrap_or(Usd::ZERO);
    let sch1_income = ri.sch1.state_refund_taxable + sum_unemployment(ri) + non_ledger_sch_c;

    // Sch 1 Part II adjustments (non-crypto): L18 early-withdrawal penalty + L21 student-loan.
    // (L15 ½-SE is crypto-Schedule-C-driven → excluded here.)
    let early_wd: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box2_early_withdrawal_penalty)
        .sum();
    // ★ `noncrypto_cap_agi` stands where a bare `cap_gain_distr` used to: the same figure whenever
    //   distributions are positive and there is no carryforward (every golden), and the
    //   §1211(b)-limited one when a broker loss makes the difference matter.
    let income_total = wages + taxable_int + ord_div + noncrypto_cap_agi + sch1_income;
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
    // ★ 1040 L15 (non-crypto) is NOT bound here any more. It was `(agi − deduction).max(0)`, and
    //   `ordinary_taxable_income` below deliberately works from the UNCLAMPED difference — see its
    //   comment for why that clamp, applied first, manufactured income out of a capital loss.
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
    // ★★★ COMPUTED FROM `agi`, NOT FROM `taxable_income`, and that is the second Critical the fold
    //     introduced. `taxable_income` is `(agi − deduction).max(0)`, already floored — so when a
    //     broker loss drove AGI below the deduction, the clamp discarded the negative and subtracting
    //     a NEGATIVE capital term then MANUFACTURED ordinary income out of nothing: W-2 $50,000 with a
    //     $200,000 broker long-term loss produced an ordinary base of $200,000 against a true
    //     $32,400, pricing every crypto ordinary dollar at 32-35% instead of 12%.
    //
    //     Subtracting the capital term from the UNCLAMPED `agi − deduction` and clamping once, at the
    //     end, is both correct and identical to the old expression on every path where `agi` clears
    //     the deduction — which is every pre-existing one, since box-2a distributions cannot be
    //     negative.
    //
    //     ★★ THE KNOWN APPROXIMATION ABOVE IS TRIGGERED BY `agi < deduction`, NOT by the preferential
    //     slice. Its own text names `TI < qd + cap_gain_distr`, and r4 measured the real region: a
    //     filer with W-2 $5,000 and unused standard deduction diverges with ZERO preferential income
    //     (engine $1,808 vs a form-derived $740). Pre-existing — the same shape reproduces on a
    //     box-2a-only vector that predates B4 entirely — and it OVERSTATES, so it is conservative in
    //     the direction that matters. Recorded here because the paragraph immediately above reasons
    //     about exactly this low-income region ("AGI can only dip $3,000 below the deduction") and
    //     would otherwise leave the next reader believing it is the only thing living there.
    let ordinary_taxable_income = (agi - deduction - qual_div - noncrypto_cap_agi).max(Usd::ZERO);

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
        // §G-28/B4 — box-2a distributions PLUS the broker long-term totals. Both are non-crypto
        // LT-character capital gain, which is precisely what this field is for.
        other_net_capital_gain: cap_gain_distr + b1099_lt,
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
/// §4.10 compute-dependent refuses that need L12/L15/L16 — now **the QBI-above-threshold rows only**
/// — are screened by [`screen_absolute`] after this (infallible) assembly. (The Form 6251 and
/// TI≤0-with-carryforward rows this list used to name are both gone: §G-6 built the AMT emitter, and
/// widening (A) lifted the carryforward refusal along with its variant.)
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
    /// The filled **Form 6251** (§4.11). Computed here because `assemble_absolute` holds the
    /// `TaxTable` (for the §1(h) breakpoints Part III reads) while `screen_absolute` takes `ar`
    /// immutably. `screen_absolute` only READS this — see `must_attach()`.
    pub amt: crate::tax::form6251::Form6251,
    /// 1040 **L12** — the deduction taken = `choose_deduction(standard, itemized)` (max, or `ForceItemize`
    /// / MFS-coupled §63(c)(6)).
    pub deduction: Usd,
    /// Whether L12 is the ITEMIZED deduction (vs the standard) — the actual §63(e)/§63(c)(6) election,
    /// for the dual-report label (not re-derivable from the amounts under `ForceItemize`/MFS coupling).
    pub deduction_is_itemized: bool,
    /// 1040 **L13** — the Form 8995 QBI deduction (REIT §199A dividends; 0 when there is no QBI).
    pub qbi_deduction: Usd,
    /// 1040 **L13b** — "Additional deductions from Schedule 1-A, line 38" (tips, overtime, vehicle
    /// loan interest, the enhanced senior deduction).
    ///
    /// ★ **Structurally zero for TY2024 because the line does not EXIST on that form** — this is not a
    /// stub for something unmodelled. The 2024 form's L14 is "Add lines 12 and 13"; the 2025 form's is
    /// "Add lines 12e, 13a, and 13b". A three-term sum with a genuinely-absent third term is the same
    /// number in 2024 and the right number in 2025. Schedule 1-A itself is `design/ty2025` B3.
    pub schedule_1a_additional: Usd,
    /// 1040 **L14** — "Add lines 12e, 13a, and 13b" (2025) / "Add lines 12 and 13" (2024):
    /// `deduction + qbi_deduction + schedule_1a_additional`.
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
    /// ★ Form 8995 **line 16** — the qualified business (loss) carryforward to next year (magnitude).
    /// For the write-back. Zero unless a prior-year QBI loss exceeded this year's business income.
    pub qbi_carryforward_out: Usd,
    /// ★★★ **N1** — the §1211/§1212 **Capital Loss Carryover Worksheet — Lines 6 and 14**, figured on
    /// this return, or `None` when the worksheet's own applicability sentence says *"you don't have
    /// any carryovers."* Every numbered line is on
    /// [`crate::tax::capital_loss_carryover::CapitalLossCarryoverWorksheet`].
    pub capital_loss_carryover_worksheet: Option<CapitalLossCarryoverWorksheet>,
    /// ★★★ **N1** — the §1212(b) capital-loss carryforward INTO next year, by character: the
    /// worksheet's lines 8 and 13.
    ///
    /// **This is NOT `TaxResult::carryforward_out`, and the difference is the defect.** The frozen
    /// crypto-delta engine computes `carry = loss − min(loss, $3,000)` flat, with no taxable-income
    /// term — exact whenever 1040 line 15 is ≥ 0, and understated by up to the whole §1211(b)
    /// allowance when the loss wiped the year out. The engine *cannot* do better: its `TaxProfile`
    /// carries `ordinary_taxable_income` already floored at zero, so the pre-floor line 15 the
    /// worksheet's line 1 needs does not survive into it. The full return has it, so the worksheet
    /// runs here.
    pub capital_loss_carryforward_out: Carryforward,
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
    /// (MAX = 6.2% × the year's SS wage base), summed over taxpayer + spouse (never pooled).
    ///
    /// ★ Requires **more than one employer** (§6413(c)); each employer contributes at most MAX before
    ///   the aggregate is compared to it. A single employer's over-withholding is $0 here and surfaces
    ///   as [`Self::excess_ss_not_creditable`] instead.
    pub excess_social_security: Usd,
    /// **Which §199A form this return files** (§G-28/B1a): `true` ⇒ Form 8995-A Part IV, `false` ⇒ the
    /// simplified Form 8995.
    ///
    /// ★★ A CORE fact, not the filler's — the same principle `packet.rs` already states for Form 8959's
    /// filing decision. Above the §199A(e)(2) threshold i8995a's "Who Must File" retires the simplified
    /// form, and that test needs `FullReturnParams`, which the printed-return assembler does not hold.
    /// Deciding it here means the printer transcribes a decision instead of re-deriving one.
    pub uses_8995a: bool,
    /// **§G-28/B1b — Form 8995-A Parts I–III**, for the same reason [`Self::uses_8995a`] lives here:
    /// Part III lines 21 and 23 are the §199A(e)(2) threshold and the phase-in range width, which come
    /// from `FullReturnParams`, and the printed-return assembler does not hold it. `None` when there is
    /// no qualified trade or business to list — a REIT/PTP-only filer, or one whose SSTB §199A(d)(3)
    /// excluded.
    pub f8995a_parts_i_to_iii: Option<crate::tax::qbi_a::Form8995APartIToIii>,
    /// Every (person, employer) pair whose Social Security withholding exceeded the §3101(a) cap —
    /// **not creditable** on this return, and therefore appearing on no line.
    ///
    /// ★★ It exists so the filer can be TOLD. i1040gi: *"The employer should adjust the tax for you. If
    /// the employer doesn't adjust the overcollection, you can file a claim for refund using Form
    /// 843."* btctax used to refuse outright here; it now files a correct $0 credit, and a conservative
    /// omission is permitted **only if the filer is told**. Drives
    /// [`crate::tax::advisories::Advisory::ExcessSsSingleEmployerNotCreditable`].
    pub excess_ss_not_creditable: Vec<NonCreditableSs>,
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

/// Schedule C's non-money header (lines A, B and F). Strings, so `PrintedInputs` is not `Copy`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScheduleCHeader {
    /// Line A — principal business or profession.
    pub business_description: String,
    /// Line B — the NAICS code.
    pub naics_code: String,
    /// Line F — `true` = accrual, `false` = cash.
    pub accrual: bool,
    /// ★ Line **I** — the filer's own answer, carried as `Option` so the writer can tell "never asked"
    /// from "answered no". See `ScheduleCLines::line_i_1099_required`.
    pub payments_requiring_1099: Option<bool>,
    /// Line **J** — answered only when line I is `Some(true)` (the form's own "If 'Yes,'").
    pub will_file_required_1099: Option<bool>,
}

/// The Form 8959 / 8960 / 8995 inputs, captured at derivation.
///
/// These are not new facts — they are the *same* values `assemble_absolute` fed to the COMPUTED
/// `Form8959` / `Form8960` / `compute_8995`. Carrying them means the printed chain and the computed tax
/// see identical inputs by construction. The alternative (re-summing Σ box 5 inside the printed chain) is
/// a second derivation, and a second derivation is exactly how a filed form comes to disagree with the
/// tax the report computed from it (SPEC §3.1 — `btctax-forms` does no arithmetic, and neither should the
/// packet).
/// **No `Default`** — deliberately, and for the same reason `AbsoluteReturn` has none: a silently-zeroed
/// field here is a wrong number on a filed return. A defaulted `capital_loss_limit` of $0 would zero the
/// §1211(b) capital-loss deduction; a defaulted `extension_payment` would re-bill a payment already made.
/// Every field is spelled out at the one construction site (and in the fixtures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedInputs {
    /// Form 8959 line 1 — Σ W-2 box 5 (household total Medicare wages).
    pub medicare_wages: Usd,
    /// Form 8959 Part V — Σ W-2 box 6 (Medicare tax withheld).
    pub medicare_withheld: Usd,
    /// Form 8960 line 7 — Σ non-business crypto **lending** interest (investment income with no home on
    /// 1040 line 2b; hobby mining/staking rewards stay OUT of NII).
    pub crypto_lending_interest: Usd,
    /// Form 8995 lines 1c/2 — the **trade-or-business QBI**: Schedule C's net profit less the §164(f)
    /// deductible half of SE tax. A crypto mining business is a qualified trade or business, and this is
    /// the base of the 20% §199A deduction it earns.
    pub business_qbi: Usd,
    /// Form 8995 line 6 — Σ 1099-DIV box 5 (§199A REIT dividends).
    pub reit_dividends: Usd,
    /// Form 8995 line 7 — the REIT/PTP loss carryforward IN.
    pub reit_ptp_carryforward_in: Usd,
    /// ★ Form 8995 line 3 — the prior-year qualified business net (loss) carryforward IN (magnitude).
    pub qbi_carryforward_in: Usd,
    /// Form 8995 line 11 — taxable income BEFORE the QBI deduction (AGI − L12).
    pub ti_before_qbi: Usd,
    /// Form 8995 line 12 — net capital gain (qualified dividends + §1(h) preferential net LTCG).
    pub qbi_net_capital_gain: Usd,
    /// Schedule C's **header lines A / B / F** — captured expressly for those cells, and printed by no
    /// one until now (ARCH-P6.3a Q7 item 6).
    pub schedule_c_header: ScheduleCHeader,
    /// 1040 **line 2a** — tax-exempt interest (Σ 1099-INT box 8 + 1099-DIV box 12). Changes no tax, but
    /// the IRS document-matches 1099-INT box 8: a return that omits it misstates itself (Q7 item 2).
    pub tax_exempt_interest: Usd,
    /// Schedule SE **line 8a** — the SE earner's OWN W-2 Social Security wages (box 3 + box 7 tips). The
    /// §1402(b)(1) cap is PER-INDIVIDUAL, so this is the owner's own wages, never the household total.
    pub se_w2_ss_wages: Usd,
    /// Schedule SE **line 7** — the year's Social Security wage base (pre-printed on the form; line 9 is
    /// "subtract line 8d from line 7").
    pub ss_wage_base: Usd,
    /// ★★★ Schedule D **line 20**'s second conjunct — *"and you are not filing Form 4952"* — as the
    /// filer declared it, so the printed chain reads an ANSWER rather than a literal `true`.
    /// `screen_inputs` refuses `None` and `Some(true)`, so a printed return always carries
    /// `Some(false)` today; carrying it anyway is what makes the checkbox's provenance structural.
    pub filing_form_4952: Option<bool>,
    /// ★★ Form 8960 **line 9b** — *"State, local, and foreign income tax"* — as the FILER allocated it
    /// (`ReturnInputs::form_8960_line9b`), or `None` when they allocated none. Carried rather than
    /// re-read so the printed chain and the absolute `form_8960` cannot end up on different values;
    /// `None` reaches `push_money_opt` and the cell prints BLANK.
    pub form_8960_line9b: Option<Usd>,
    /// ★★★ **1040 line 19** — *"Child tax credit or credit for other dependents from Schedule 8812"*,
    /// as [`crate::tax::advisories::ctc_odc_line19`] decides it: `Some(0)` only where Schedule 8812
    /// line 12 provably answers **No** (its own instruction is *"Enter -0- on lines 14 and 27"*, and
    /// line 14 routes that figure to this line), `None` everywhere else, where btctax figures no §24
    /// credit and the cell is the FILER's to fill. `None` reaches `push_money_opt` and prints BLANK.
    ///
    /// ★★ Carried rather than re-derived in `printed.rs`, for the same reason as `form_8960_line9b`
    /// above and one more: `form_1040_lines` holds no `ReturnInputs`, and giving the printed lane its
    /// own §24(b) predicate would be a second divergent implementation of the rule. FR-1.
    pub ctc_odc_line19: Option<Usd>,
    /// Schedule D **line 21** — the §1211(b) capital-loss deduction CEILING ($3,000; $1,500 MFS). The
    /// printed line 21 caps the PRINTED line 16 against this, rather than re-rounding the exact
    /// deduction, so the filed Schedule D's own arithmetic holds.
    pub capital_loss_limit: Usd,
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
    // §G-28/B3 — non-ledger Schedule C revenue. Zero unless the filer stated it; see the field's doc
    // for why a plain `Usd` is safe here (the §G-22 out-of-scope-income declaration is the guard).
    let other_gross_receipts = ri
        .schedule_c
        .as_ref()
        .map(|c| c.other_gross_receipts)
        .unwrap_or(Usd::ZERO);
    // ★★★ §G-28/B3 — NON-LEDGER RECEIPTS REACH SCHEDULE SE THROUGH THE EXPENSES ARGUMENT, and that
    //     needs a proof rather than a shrug, because `compute_se_tax` lives in the FROZEN delta engine
    //     (`frozen_guard.rs`; SPEC_full_return §2 — the full-return build wraps it and never edits it).
    //
    //     THE PROOF. `schedule_c_expenses` has exactly ONE arithmetic use in that function:
    //
    //         let gross_se = se_net_income(state, year);
    //         let n = gross_se - schedule_c_expenses;      // net SE earnings, then `.max(0)`
    //
    //     Substituting `expenses − receipts` gives `n = gross_se − expenses + receipts`, which IS
    //     `(ledger gross + non-ledger receipts) − expenses` — Schedule C line 1 minus line 28,
    //     identically. Every later term (the 92.35% factor, the §1401(a) cap, §1401(b)) reads only
    //     `n`, so nothing else can differ.
    //
    //     ★★ THE BRANCH WHERE IT BREAKS: if `compute_se_tax` ever reads `schedule_c_expenses` a
    //     SECOND time — say to print it, or to floor it at zero independently — the substitution stops
    //     being a substitution. `frozen_guard` pins that file's SHA-256, so such an edit cannot land
    //     silently; and `non_ledger_receipts_reach_schedule_se_exactly` is the KAT that pins the
    //     equality itself.
    //
    //     ★★★ AND THE SPECIFIC EDIT TO EXPECT IS A CLAMP, because that file's OWN DOC invites it:
    //         *"`schedule_c_expenses`: … **Must be ≥ 0** — the CLI validates; this function assumes
    //         the precondition holds."* That precondition is now FALSE — this call passes a negative
    //         whenever receipts exceed expenses — so a well-meaning hardening pass adding
    //         `schedule_c_expenses.max(Usd::ZERO)` would silently truncate every such filer's SE base
    //         to the ledger half. On the KAT's own vector that is $40,000 instead of $120,000, about
    //         **$10,400 of understated SE tax**. `frozen_guard`'s exception process explicitly permits
    //         that edit in its own reviewed commit, so the KAT — not the freeze — is what stops it.
    //         Anyone taking that exception must read this comment first.
    //
    //     ★ The DELTA engine still cannot see these receipts — `TaxProfile` has no field for them and
    //     it is frozen too. That understates the §1401(a) band it thinks is available, so the
    //     crypto-ATTRIBUTABLE SE tax it reports comes out too high (the safe direction, and already
    //     within the report's disclosed ceteris-paribus limits). Filed as §G-30.
    let se = compute_se_tax(
        state,
        year,
        status,
        table,
        w2_ss_wages,
        w2_medicare_wages,
        schedule_c_expenses - other_gross_receipts,
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
    // §G-28/B4 — Schedule D lines 1a and 8a, columns (d) and (e). Summed here, printed there.
    let (b1099_st_proceeds, b1099_st_basis, b1099_lt_proceeds, b1099_lt_basis) =
        ri.b_1099.iter().fold(
            (Usd::ZERO, Usd::ZERO, Usd::ZERO, Usd::ZERO),
            |(sp, sb, lp, lb), b| {
                (
                    sp + b.short_term_proceeds,
                    sb + b.short_term_basis,
                    lp + b.long_term_proceeds,
                    lb + b.long_term_basis,
                )
            },
        );
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
        st_1099b_proceeds_1ad: b1099_st_proceeds,
        st_1099b_cost_1ae: b1099_st_basis,
        // ★ Kept as the exact difference, but note the PRINTED form does not read it: `schedule_d_lines`
        //   recomputes column (h) from the two ROUNDED cells, because "Subtract column (e) from column
        //   (d)" must check out for a reader of the filed page. This field is the unrounded truth the
        //   tests assert against; the two agree at every whole-dollar input, which is all of them.
        st_1099b_gain_1ah: b1099_st_proceeds - b1099_st_basis,
        lt_1099b_proceeds_8ad: b1099_lt_proceeds,
        lt_1099b_cost_8ae: b1099_lt_basis,
        lt_1099b_gain_8ah: b1099_lt_proceeds - b1099_lt_basis,
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
    // ★★★ §G-28/B3 — SCHEDULE C LINE 1, and the ONE place it is formed. The ledger's SE-eligible
    //     business crypto PLUS the filer's non-ledger receipts. Everything below reads this: net
    //     profit, Schedule 1 line 3, Schedule SE, and the §199A QBI base.
    let schedule_c_gross = crypto.business_se_gross + other_gross_receipts;
    let schedule_c_net = (schedule_c_gross - schedule_c_expenses).max(Usd::ZERO);
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

    // Schedule C exists whenever the filer declared a trade or business. Gross receipts (line 1) are
    // the SE-eligible business crypto income PLUS any non-ledger receipts (§G-28/B3); the net (floored
    // at 0 — a loss refuses upstream) is the SAME figure that feeds Schedule 1 line 3 and Schedule SE.
    let schedule_c = ri.schedule_c.as_ref().map(|_| ScheduleCParts {
        gross_receipts_1: schedule_c_gross,
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

    // QBI / Form 8995 (L13) — BOTH components: the crypto Schedule C trade or business AND §199A REIT
    // dividends. The §199A(e)(2)-above-threshold refuse is compute-dependent → `screen_absolute` (this
    // assembly is infallible best-effort; the screen gates the report before the number is used).
    //
    // ★ **QBI = Schedule C's net profit MINUS the §164(f) deductible half of SE tax.** The Form 8995
    // instructions define QBI net of the deductible part of SE tax, self-employed health insurance and
    // self-employed retirement contributions — v1 models only the first (the others have no input). A
    // crypto MINING trade or business is a qualified trade or business (not an SSTB), so its owner is
    // entitled to the deduction; v1 originally computed the REIT component only, which silently
    // OVERSTATED a miner's tax by ~20% of their business income. The P7 independent-oracle cross-check
    // found it (the PSL Tax-Calculator applies the deduction and btctax did not), and it confirms this
    // exact rule to the dollar: $60,000 profit − $4,239 half-SE = $55,761 of QBI ⇒ an $11,152 deduction.
    // 1040 L13b — "Additional deductions from Schedule 1-A, line 38". Zero until Schedule 1-A lands
    // (design/ty2025 B3); the 2024 form has no such line, so zero is the RIGHT value there, not a stub.
    //
    // ★ Defined HERE rather than beside line 14 below, because TI-before-QBI subtracts it and the
    //   §199A regime is decided from TI-before-QBI — the ordering is load-bearing, not cosmetic.
    let schedule_1a_additional = Usd::ZERO;

    // ★★★ TAXABLE INCOME BEFORE THE QBI DEDUCTION — defined ONCE, here, and read by everything.
    //
    //     i8995a: *"Form 1040 … filers: Form 1040 … line 11, minus Form 1040 … line 12."* On the
    //     TY2025 form that subtrahend is 12 **and 13b** (Schedule 1-A), which is why the third term is
    //     here and not just `agi - deduction`.
    //
    //     ★★ It prints on the SAME PAGE TWICE — Form 8995-A line 20 (Part III) and line 33 (Part IV) —
    //     and it also decides the §199A regime, the form choice, and the refusal. B1b's first draft
    //     had the regime, the form choice and line 20 reading `agi - deduction` while line 33 read the
    //     13b-adjusted figure. That is dormant only while `schedule_1a_additional` is zero: the moment
    //     Schedule 1-A lands, a filer with 13b deductions is classified into the wrong regime, Part III
    //     is skipped when it should run, and the emitted form prints line 20 ≠ line 33.
    //
    //     ★ ROUNDED at the source, so the DECISION and the PAGE agree. A hand-filer rounding to whole
    //     dollars (SPEC §3.1) compares the rounded figure against the threshold, and a return whose
    //     line 20 prints $191,950 against a line 21 of $191,950 must not simultaneously claim to be
    //     *"more than $191,950"*. Rounding after the test made the page contradict its own gate.
    let ti_before_qbi = round_dollar(agi - deduction - schedule_1a_additional);

    let business_qbi_before_sstb =
        (schedule_c.as_ref().map_or(Usd::ZERO, |c| c.net_profit_31) - half_se).max(Usd::ZERO);
    // ★★★ §G-28/B1b — §199A(d)(3): above the phase-in range a SPECIFIED SERVICE trade or business is
    //     not a qualified trade or business at all, so *"no QBI, W-2 wages, or UBIA of qualified
    //     property from the specified service trade or business are taken into account"* (i8995a).
    //
    //     ★★ Applied HERE, at the one place `business_qbi` is born, so the Form 8995 chain, Form
    //     8995-A Parts I–III and the refusal screen all read the same figure. Applying it inside the
    //     8995-A emitter instead would leave Form 8995 and Schedule SE reading an un-excluded QBI —
    //     the two-authorities-for-one-number defect this codebase keeps finding.
    let business_qbi = qbi_after_sstb_exclusion(
        business_qbi_before_sstb,
        // ★ Read off the INPUTS, and gated on the assembled Schedule C existing at all — a return
        //   whose business assembled to nothing has no QBI for the exclusion to bite on.
        schedule_c.is_some() && ri.schedule_c.as_ref().and_then(|c| c.is_sstb) == Some(true),
        Qbi199aRegime::of(ti_before_qbi, ri.filing_status, params),
    );
    let reit_dividends: Usd = ri.div_1099.iter().map(|d| d.box5_section_199a).sum();
    let net_capital_gain = qualified_dividends + net_ltcg; // Form 8995 line 12

    // §G-28/B1a — the §199A FORM choice. See the field's doc for why it is decided here.
    let uses_8995a = crate::tax::qbi::uses_8995a(
        business_qbi,
        reit_dividends,
        ri.qbi.reit_ptp_carryforward_in,
        ri.qbi.qbi_carryforward_in,
        ti_before_qbi,
        status,
        params,
    );
    // §G-28/B1b — Parts I–III, decided here for the same reason: lines 21 and 23 need `params`.
    //
    // ★ Built from `business_qbi` AFTER the §199A(d)(3) SSTB exclusion above, so an excluded SSTB has
    //   no qualified business and `compute` returns `None` — Parts I–III are then not filed at all,
    //   which is what the statute means by "not a qualified trade or business".
    let f8995a_parts_i_to_iii = uses_8995a
        .then(|| {
            crate::tax::qbi_a::Form8995APartIToIii::compute(
                &crate::tax::qbi_a::PartIToIiiInputs {
                    // ★ TRIMMED, like `schedule_c_header.business_description` below — the same
                    //   business must not be named two ways in one packet (Schedule C line A and
                    //   Form 8995 row 1i(a) already share the canonical form). Core refuses an
                    //   all-whitespace name, so this only ever strips surrounding padding.
                    business_name: ri
                        .schedule_c
                        .as_ref()
                        .map(|c| c.business_description.trim().to_string())
                        .unwrap_or_default(),
                    is_sstb: ri.schedule_c.as_ref().and_then(|c| c.is_sstb) == Some(true),
                    business_qbi,
                    // ★ `unwrap_or(ZERO)` is safe ONLY because `screen_absolute` refuses an
                    //   unanswered pair above the threshold, and `uses_8995a` is false below it. A
                    //   defaulted zero here would otherwise CAP the deduction at zero for a filer who
                    //   does pay wages, which overstates their tax.
                    w2_wages: ri
                        .schedule_c
                        .as_ref()
                        .and_then(|c| c.qbi_w2_wages)
                        .unwrap_or(Usd::ZERO),
                    ubia: ri
                        .schedule_c
                        .as_ref()
                        .and_then(|c| c.qbi_ubia)
                        .unwrap_or(Usd::ZERO),
                    ti_before_qbi,
                },
                Qbi199aRegime::of(ti_before_qbi, status, params),
                status,
                params,
            )
        })
        .flatten();
    // ★★★ §G-28/B1b — THE QUALIFIED BUSINESS COMPONENT, and the ONE place it is decided.
    //
    //     On the Form 8995-A path it is Part II line 16 — 20% of QBI AFTER the §199A(b)(2)
    //     W-2-wage/UBIA cap and the Part III phase-in. Letting `compute_8995` recompute a flat 20%
    //     here would carry an UNCAPPED deduction onto the 1040 while the attached Form 8995-A printed
    //     the capped figure on its own line 16 — the two-authorities-for-one-number defect, in the
    //     UNDERSTATING direction.
    //
    //     ★ `None` when there is no qualified trade or business (REIT/PTP only, or an SSTB the
    //       §199A(d)(3) exclusion removed): `business_qbi` is then zero and the flat 20% of zero is
    //       the same answer, so the simplified path is not merely safe there, it is identical.
    let qbi_component_8995a = f8995a_parts_i_to_iii.as_ref().map(|f| f.part_ii.line16);

    let qbi = compute_8995(
        business_qbi,
        qbi_component_8995a,
        reit_dividends,
        ri.qbi.reit_ptp_carryforward_in,
        ri.qbi.qbi_carryforward_in, // Form 8995 line 3 (magnitude; line 4 subtracts it)
        ti_before_qbi,              // Form 8995 line 11 = TI before the QBI deduction
        net_capital_gain,
    );
    // L14 — "Add lines 12e, 13a, and 13b" (2025) / "Add lines 12 and 13" (2024).
    let total_deductions = deduction + qbi.deduction + schedule_1a_additional;
    // ★★★ **N1** — 1040 L15, and the SIGNED figure the §1211/§1212 Capital Loss Carryover Worksheet's
    //     line 1 asks for. The filed line is floored at zero; the worksheet explicitly wants the
    //     amount that line "would have been … if you could enter a negative number on that line", and
    //     computing the carryforward-out from the floored one understates the surviving loss by up to
    //     the whole §1211(b) allowance. Both are kept, and the unfloored one is used ONLY here.
    let taxable_income_signed = agi - total_deductions;
    let taxable_income = taxable_income_signed.max(Usd::ZERO); // L15

    // The worksheet reads five figures off the return, all of which now exist: the signed L15 above
    // and four Schedule D lines already assembled from the same `cap` netting the printed form uses.
    let capital_loss_carryover_worksheet =
        CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: taxable_income_signed,
            schedule_d_line7: schedule_d.st_net_7,
            schedule_d_line15: schedule_d.lt_net_15,
            schedule_d_line16: schedule_d.total_16,
            schedule_d_line21_loss: schedule_d.loss_deduction_21,
        });
    let capital_loss_carryforward_out = capital_loss_carryover_worksheet
        .map(|w| w.carryforward_out())
        .unwrap_or_default();

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
        // Part II line 9b — the filer's own §1411(c)(1)(B) allocation, `None` when they made none.
        // Bounded by `screen_absolute` (`Nii9bExceedsDeductedSalt`), never clamped here: a silently
        // shrunk figure would be btctax choosing the allocation method it must not choose.
        ri.form_8960_line9b,
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
    // ★★★ §G-6 — FORM 6251 IS COMPUTED **HERE**, ABOVE L18, BECAUSE L18 CONSUMES IT.
    //
    //     It used to run below `total_tax`, and `l18` was hardcoded to `regular_tax` with a comment
    //     citing the AMT REFUSAL as its warrant. Removing that refusal made the comment false and left
    //     `AbsoluteReturn::total_tax` short by exactly the AMT — an UNDERSTATEMENT. It was invisible
    //     because the printed chain is assembled separately and was fixed first, and because nothing
    //     outside this function reads `total_tax` today. "Nothing reads it yet" is not a guarantee; a
    //     wrong public figure is a trap with a fuse on it.
    //
    //     ★ `compute_6251` needs `foreign_tax_credit` (line 8, the AMTFTC input), so it cannot move any
    //       higher than this — which is exactly why the ordering was wrong in the first place.
    let amt = crate::tax::form6251::compute_6251(
        form6251_inputs_from_parts(
            ri,
            agi,
            taxable_income,
            deduction,
            qbi.deduction,
            deduction_is_itemized,
            schedule_a.as_ref(),
            qualified_dividends,
            net_ltcg,
            regular_tax,
            foreign_tax_credit,
        ),
        &params.amt,
        // ★ `ltcg_for`, NOT a raw `ltcg.get(&status)`: the map carries no `Qss` key (the adapters note
        // "QSS is not inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time"), so the raw
        // lookup aborts the process for every Qualifying-Surviving-Spouse return. Same accessor the
        // regular-tax call uses above.
        table.ltcg_for(status),
    );

    // Sch 2 Part I: L1z (excess-APTC) = 0 (no input); L2 (AMT) = Form 6251 line 11. So L17 = Sch 2 L3
    // = L1z + L2 = the AMT, and L18 = L16 + L17.
    //
    // ★ Unconditional, matching `printed.rs`: line 11 is $0 whenever no AMT is owed, and a positive
    //   line 11 implies Who-Must-File condition 1 (line 11 = line 7 − line 8 − line 10 with line 8 ≥ 0,
    //   so line 11 > 0 ⇒ line 7 > line 10). The printed blank on Schedule 2 line 2 is a PRESENTATION
    //   decision about an absent form; it never drops a non-zero figure.
    let l18 = regular_tax + amt.line11; // L16 + L17
    let nonrefundable_credits = ctc_odc_credit + foreign_tax_credit; // L21 = L19 + L20 (v1: FTC only)
    let tax_after_credits = (l18 - nonrefundable_credits).max(Usd::ZERO); // L22
                                                                          // Sch 2 Part II (L21) → 1040 L23 = SE (L4) + Additional Medicare (L11) + NIIT (L12).
    let schedule_2_other_taxes =
        se_tax_sch2_l4 + additional_medicare.additional_medicare_tax + niit.tax;
    let total_tax = tax_after_credits + schedule_2_other_taxes; // L24

    // ── Excess-SS + payments → refund/owed (SPEC §5 stages 8–9) ─────────────────────────────────────
    let excess_social_security = excess_social_security(ri, table);
    let excess_ss_not_creditable = non_creditable_ss(ri, table);

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
        schedule_1a_additional,
        amt,
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
        qbi_carryforward_out: qbi.qbi_carryforward_out,
        capital_loss_carryover_worksheet,
        capital_loss_carryforward_out,
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
        excess_ss_not_creditable,
        uses_8995a,
        f8995a_parts_i_to_iii,
        withholding_25a: wh_25a,
        withholding_25b: wh_25b,
        total_withholding,
        total_payments,
        overpayment_refund,
        amount_owed,
        // The printed chains read THESE — the same values the computed 8959/8960/8995 above were fed.
        printed_inputs: PrintedInputs {
            // ★ Schedule D line 20's Form 4952 conjunct, carried from the FILER's answer.
            filing_form_4952: ri.filing_form_4952,
            // ★ Form 8960 line 9b, likewise carried rather than derived — the same `Option<Usd>` the
            //   absolute `form_8960` above was handed.
            form_8960_line9b: ri.form_8960_line9b,
            // ★★★ FR-1 — 1040 line 19. Decided HERE because this is where `ri` and the AGI meet, and
            //     decided by the same call `advisories_for` makes with the same `agi`, so the printed
            //     cell and the `CtcOdcOmitted` advisory cannot contradict each other on one packet.
            ctc_odc_line19: crate::tax::advisories::ctc_odc_line19(ri, agi),
            medicare_wages: w2_medicare_wages,
            medicare_withheld: w2_medicare_withheld,
            crypto_lending_interest: crypto.nonbusiness_lending_interest,
            business_qbi,
            reit_dividends,
            reit_ptp_carryforward_in: ri.qbi.reit_ptp_carryforward_in,
            qbi_carryforward_in: ri.qbi.qbi_carryforward_in,
            // Form 8995 line 11, "Taxable income before qualified business income deduction" — the
            // 2025 i8995 figures it from 1040 line 11a MINUS lines 12e and 13b. Note it excludes
            // 13a, which IS the QBI deduction being computed. Omitting 13b would OVERSTATE this,
            // inflating the §199A deduction and firing `qbi_over_threshold` too EARLY: a false
            // refusal. Zero today because Schedule 1-A is B3.
            ti_before_qbi,
            qbi_net_capital_gain: net_capital_gain,
            schedule_c_header: ri
                .schedule_c
                .as_ref()
                .map_or_else(ScheduleCHeader::default, |c| ScheduleCHeader {
                    // Trimmed ONCE, here, so Schedule C line A and Form 8995 row 1i(a) carry the
                    // same canonical string (Fable P7 r3, Minor). Core refuses an all-whitespace name,
                    // so this only ever strips surrounding padding.
                    business_description: c.business_description.trim().to_string(),
                    naics_code: c.naics_code.clone(),
                    accrual: c.accounting_method
                        == crate::tax::return_inputs::AccountingMethod::Accrual,
                    payments_requiring_1099: c.payments_requiring_1099,
                    will_file_required_1099: c.will_file_required_1099,
                }),
            tax_exempt_interest: ri
                .int_1099
                .iter()
                .map(|i| i.box8_tax_exempt_interest)
                .chain(
                    ri.div_1099
                        .iter()
                        .map(|d| d.box12_exempt_interest_dividends),
                )
                .sum(),
            se_w2_ss_wages: w2_ss_wages,
            ss_wage_base: table.ss_wage_base,
            capital_loss_limit: loss_limit(status),
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
///
/// ★ `pub(crate)` for [`crate::tax::scrub::ledger_contributes`], which is the ONLY other caller.
/// That predicate uses this as one disjunct of four and must not be confused with it: reading a
/// `false` here as "the ledger is empty" is exactly the widening `ledger_contributes` exists to
/// prevent (SPEC_income_scrub.md §2.2).
pub(crate) fn digital_asset_activity(state: &LedgerState, year: i32) -> bool {
    state.disposals.iter().any(|d| d.disposed_at.year() == year)
        || state
            .income_recognized
            .iter()
            .any(|i| i.recognized_at.year() == year)
        || state.removals.iter().any(|r| r.removed_at.year() == year)
}

/// Build [`crate::tax::form6251::Form6251Inputs`] from the pieces `assemble_absolute` holds before it
/// has an `AbsoluteReturn` to hand.
///
/// ★ Lines 2a and 1 cite DIFFERENT 1040 lines — 12 and 14 — so both are passed. `qdcgt_line5_regular`
/// is the QDCGT Worksheet's line 5 **as figured for the regular tax** (Form 6251 lines 20 and 27).
#[allow(clippy::too_many_arguments)]
pub(crate) fn form6251_inputs_from_parts(
    ri: &ReturnInputs,
    agi: Usd,
    taxable_income: Usd,
    deduction: Usd,
    qbi_deduction: Usd,
    itemized: bool,
    schedule_a: Option<&ScheduleAParts>,
    qualified_dividends: Usd,
    net_ltcg: Usd,
    regular_tax: Usd,
    foreign_tax_credit: Usd,
) -> crate::tax::form6251::Form6251Inputs {
    let pref = (qualified_dividends + net_ltcg).max(Usd::ZERO);
    crate::tax::form6251::Form6251Inputs {
        // TY2024's Part I. TY2025 passes `Y2025 { .. }` here; the year lives at THIS call site,
        // which is the one place that knows it (D-4).
        line1_rule: crate::tax::form6251::Form6251Line1Rule::Y2024,
        status: ri.filing_status,
        taxable_income_l15: taxable_income,
        agi_l11: agi,
        deduction_l12: deduction,
        deduction_l14: deduction + qbi_deduction,
        schedule_a_line7: schedule_a.map_or(Usd::ZERO, |p| p.salt_5e),
        itemized,
        state_refund_sch1_l1: ri.sch1.state_refund_taxable,
        net_capital_gain: net_ltcg,
        qualified_dividends,
        qdcgt_line5_regular: (taxable_income - pref.min(taxable_income)).max(Usd::ZERO),
        regular_tax_l16: regular_tax,
        schedule_2_line1z: Usd::ZERO,
        schedule_3_line1: foreign_tax_credit,
    }
}

/// Screen the **assembled-return** refuse rows (SPEC §4.10) — those that need the computed deduction /
/// taxable income, so they run AFTER [`assemble_absolute`] (which is infallible). Complements
/// [`crate::tax::return_refuse::screen_inputs`] (input-screenable) and [`screen_compute_dependent`]
/// (income/ledger-dependent). Returns the FIRST [`Refusal`], or `None`.
///
/// Rows: **(a) the §199A rows only** — an SSTB inside the phase-in range, and Form 8995-A lines 4/7
/// unstated above the threshold (§4.5).
///
/// ★★ **The TI≤0-with-carryforward row is GONE** (widening (A)). It read *"(b) taxable income ≤ 0
/// WITH a capital-loss carryforward-in — the G22 §1211/§1212 Capital Loss Carryover Worksheet
/// edge"*, and it was deleted together with its `RefuseReason` variant, so no consumer maps it and
/// no anchor names it. Such a year now **FILES**: `capital_loss_carryover` models the worksheet, so
/// the carryover-out is a real figure rather than the flat rule that made the refusal honest.
/// (The refund-only TI≤0 filer with NO carryforward was never refused either — tax = 0, withholding
/// refunded, the r5-narrowed rule.)
///
/// ★ There is NO LONGER an AMT row here either. It read *"Form 6251 Who Must File condition 1 — the
/// form must be attached and v1 cannot yet file it"*; §G-6 built the emitter, so the form is filed
/// and the row was deleted from this function's body. `RefuseReason::AmtScreenTriggered` survives as
/// a dead variant pending its Tier-2 rename.
pub fn screen_absolute(
    ri: &ReturnInputs,
    ar: &AbsoluteReturn,
    params: &FullReturnParams,
    state: &LedgerState,
    year: i32,
) -> Option<Refusal> {
    // ★★★ §G-21 — Form 8283 Section B lines 5a/5b/5c, the restriction questions.
    //
    // ★★ r3 I-2/I-3 put this HERE rather than in `screen_compute_dependent`, and re-keyed it. It was
    // gated on `year_donation_deduction > $5,000` — the Form 8283 SECTION split — which is the wrong
    // predicate in BOTH directions:
    //
    //   • TOO NARROW. §170(f)(11)(C)'s $5,000 decides which SECTION of the 8283 you file. It has
    //     nothing to do with whether a restricted gift's deduction is allowable — Reg §1.170A-7 and
    //     §170(f)(3)(A) bite at every dollar. So a filer who DECLARED a restriction on a $4,000
    //     donation had their declaration discarded and the full FMV deducted: an UNDERSTATEMENT on
    //     facts btctax had collected.
    //   • TOO WIDE. `year_donation_deduction` reads the LEDGER, not the return. A standard-deduction
    //     filer claims no §170 deduction and attaches no 8283 (`packet.rs`: "a standard-deduction year
    //     with donations files none"), so a restriction changes no figure — yet they were refused,
    //     unescapably, by a message asserting "this year files a Form 8283 SECTION B". It does not.
    //
    // The predicate is now the ELECTION the return actually made. `ar.deduction_is_itemized` is the
    // real §63(e) choice, which is why this screens AFTER `assemble_absolute` — it is not derivable
    // from the inputs, and re-deriving `choose_deduction` here is exactly the compression the
    // transcribe rule forbids.
    //
    // ★★★ FOLD-REVIEW — KEYED ON THE QUANTITY `packet.rs` ITSELF FILTERS ON, so the gate and the
    // packet cannot disagree about whether a Form 8283 exists. r3 re-keyed this once and only got
    // half of it: `deduction_is_itemized` reads the RETURN for the election, but the amount was still
    // `year_donation_deduction`, i.e. still the LEDGER — the very thing r3's own rationale above
    // condemns. An itemizing filer whose §170(b) ceiling zeroes the noncash deduction (itemizing on
    // mortgage interest, 30% of a small AGI allowing nothing) claims $0 of charity and attaches NO
    // 8283, yet was hard-blocked, escapable only by a false "No" under §6065 or by deleting a
    // truthful ledger event.
    //
    // Schedule A **line 12** is the §170(b)-LIMITED figure that actually reduces taxable income, and
    // it is what Form 8283's own text keys on: "Attach one or more Forms 8283 to your tax return if
    // you claimed a total deduction of over $500 for all contributed property" — the CLAIMED
    // deduction, not the ledger's fair market value.
    //
    // ★ A ceiling-zeroed year is NOT thereby unguarded: its excess rolls forward, and
    // `apply_carryover_writeback`'s vouch-for gate refuses to persist it. The year files clean
    // because it claims nothing; the carryover cannot be laundered. The two gates are a pair.
    let claimed_noncash = ar
        .schedule_a
        .as_ref()
        .map_or(Usd::ZERO, |a| a.charitable_noncash_12);
    let donated = crate::forms::year_donation_deduction(state, year);
    if claimed_noncash > Usd::ZERO {
        // A DECLARED restriction shrinks or denies the deduction at ANY amount.
        if ri.donations_had_restrictions == Some(true) {
            return refusal(
                RefuseReason::DonationRestrictionsUnresolved,
                "you declared that at least one donated property had a restriction or a retained \
                 right (Form 8283 line 5a, 5b or 5c). Under Reg §1.170A-7 that REDUCES or DENIES \
                 the §170 deduction, and btctax values every donation at full fair market value — \
                 so the deduction it would compute is too large. It cannot tell which gift is \
                 affected, so it will not file the year: complete Form 8283 for the restricted \
                 donation by hand, with the reduced amount",
            );
        }
        // UNANSWERED refuses only when the form actually PRINTS 5a/5b/5c — i.e. a Section B year.
        // Below that the questions are never posed, so silence forgoes nothing and asserts nothing.
        // ★ 5a/5b/5c are printed only when an 8283 ACTUALLY ATTACHES (`packet.rs` filters on line 12
        //   over $500) **and** the year is a Section B one (`forms.rs` splits on the year aggregate
        //   over $5,000). Both terms, or the message asserts a form the packet does not write.
        if ri.donations_had_restrictions.is_none()
            && claimed_noncash > crate::tax::printed::FORM_8283_THRESHOLD
            && donated > crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD
        {
            return refusal(
                RefuseReason::DonationRestrictionsUnresolved,
                "this year files a Form 8283 SECTION B (donations over $5,000), whose lines 5a, 5b \
                 and 5c ask whether any donated property carried a restriction or a retained right. \
                 A \"Yes\" to any of them reduces or denies the §170 deduction (Reg §1.170A-7), and \
                 btctax deducts at full fair market value — so it cannot file this return without \
                 the answer. Run `btctax income answer`",
            );
        }
    }

    // ★★★ §163(h)(3)(B) — THE ACQUISITION-DEBT CEILING, ANSWERED ADVERSELY (P1 / adjudication D3).
    //
    // ★★ THIS LIVES HERE, NOT IN `screen_inputs`, AND THE MOVE WAS A CRITICAL FIX (phase-2 review R2).
    //    The refusal conditions a Schedule A DEDUCTION, and `screen_inputs` cannot see the §63(e)
    //    election — only `assemble_absolute` computes it. Sitting there, it refused the
    //    December-closing jumbo homebuyer: one month of 1098 interest on a $1M post-2017 loan, an
    //    itemized total under the MFJ standard deduction, line 8a never printed, nothing sworn — and
    //    NO honest answer available, because `None` refused as unanswered, `Some(false)` refused
    //    here, and `Some(true)` would have been false testimony under §6065.
    //
    // ★★★ WHY `deduction_is_itemized` IS EXACTLY THE RIGHT PREDICATE, and not merely a convenient one.
    //     It is computed against the FULL (uncapped) Schedule A amounts. So:
    //       - itemized-with-full-amount < standard  ⇒ the election is STANDARD under both hypotheses
    //         (the true capped figure is smaller, which only widens the gap). Determinate ⇒ safe to
    //         compute, and refusing would be wrong.
    //       - itemized-with-full-amount ≥ standard  ⇒ btctax cannot know the capped figure, so it
    //         cannot know the election either. Indeterminate ⇒ refuse, which is what happens.
    //     The predicate answers the question "is the election determinate without the number btctax
    //     is missing?" — which is the actual question, and it answers it exactly.
    //
    // ★★★ …EXCEPT WHERE LINE 8a IS ALREADY A DISCLOSED ZERO (final whole-branch review, finding 3).
    //
    // A MIXED-USE mortgage (`mortgage_all_used_to_buy_build_improve == Some(false)`, §2.7, the base
    // question tree) already zeroes 8a and CHECKS THE LINE-8 BOX — a disclosed conservative zero.
    // For that filer both premises the refusal states above are FALSE:
    //
    //   * *"deducting the full Form 1098 figure would UNDERSTATE your tax"* — btctax would deduct
    //     **$0**, not the full figure. The whole 1098 amount is already forgone.
    //   * *"Schedule A has no box that would disclose such a zero"* — the box **IS** checked, and
    //     truthfully, on this very return.
    //
    // And the over-limit fact moves nothing: the return btctax prints for them (8a = $0, box
    // checked) is byte-identical to the one it prints when the same filer answers the debt-limit
    // question YES. Reproduced across all four combinations before this scope was added — the
    // mixed-use filer got `MortgageOverDebtLimit` on `Some(false)` and
    // `MortgageDebtLimitUnanswered` on `None`, leaving `Some(true)` (false testimony under §6065)
    // as the only route to a return btctax could already compute honestly.
    //
    // ★ THE SEAM, AND WHY NO EARLIER ROUND SAW IT. The mixed-use question belongs to the base tree;
    //   the debt-limit refusal is phase 1, reviewed against its own population. `screen_inputs`'s
    //   own comment ("unlike the mixed-use zero it has no line-8 checkbox disclosing it") shows the
    //   author HELD the distinction and reasoned only about the non-mixed-use filer. No window
    //   contained both.
    //
    // ★★ The refusal is unchanged for every OTHER over-limit itemizer — where 8a really would carry
    //    the full 1098 figure with nothing on the form to disclose it, which is the case its text
    //    describes and the case it exists for. Scoping it here also makes that text TRUE of everyone
    //    who now sees it.
    if ar.deduction_is_itemized
        && mixed_use_mortgage_forgone(ri).is_none()
        && crate::tax::questions::question_is_live(
            crate::tax::questions::QuestionId::MortgageWithinDebtLimit,
            ri,
        )
        && ri
            .schedule_a
            .as_ref()
            .is_some_and(|a| a.mortgage_within_debt_limit == Some(false))
    {
        return refusal(
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
             — so until it lands, file this year's Schedule A by hand. `btctax report` still runs. \
             (This return itemizes; a return that takes the standard deduction is unaffected and \
             computes normally, because line 8a never prints on it.)",
        );
    }

    // ★★★ i4952's NO-FILING EXCEPTION — the Schedule A line-9 BOUND (P7).
    //
    // ★★ ALSO MOVED HERE FROM `screen_inputs` (phase-2 review R5a, the same Critical). Same root
    //    cause, and a worse population: the crypto-margin renter is this product's CORE audience.
    //    Bitcoin yields no interest and no ordinary dividends, so the ceiling is ~$0 and ANY line-9
    //    entry with a truthful "not filing Form 4952" was refused — including when SALT plus margin
    //    interest lose to the standard deduction and the correct return claims nothing at all.
    //
    // ★ NOTE the `Some(true)` leg is NOT here — it stays in `screen_inputs`, correctly. Filing Form
    //   4952 sends Schedule D line 20 to NO and routes the tax through the Schedule D Tax Worksheet,
    //   which is a CAPITAL-GAINS consequence and applies whether or not the filer itemizes.
    if ar.deduction_is_itemized && ri.filing_form_4952 == Some(false) {
        let investment_interest = ri
            .schedule_a
            .as_ref()
            .map_or(Usd::ZERO, |a| a.investment_interest);
        if investment_interest > Usd::ZERO {
            // i4952's first exception condition: investment income from INTEREST and ORDINARY
            // dividends, minus any QUALIFIED dividends. Transcribed, not approximated.
            //
            // ★★ box 1 + box 3 (phase-2 review R5b). Form 1099-INT box 3 is US Treasury obligation
            //    interest and is NOT a subset of box 1 — `sum_taxable_interest` says so in terms, and
            //    1040 line 2b is box 1 + box 3. Summing box 1 alone under-counted the ceiling and
            //    refused a filer with a T-bill ladder who plainly satisfies the exception. Treasury
            //    interest is unambiguously "investment income from interest".
            //
            // ★ Deliberately EXCLUDED: crypto lending interest. btctax treats it as interest-like for
            //   §1411, but its character as "interest" under §163(d) is genuinely arguable, so leaving
            //   it out of the ceiling is conservative in the over-refusing (not understating)
            //   direction. Stated so the omission is not silent.
            let interest: Usd = ri
                .int_1099
                .iter()
                .map(|i| i.box1_interest + i.box3_treasury_interest)
                .sum();
            let ordinary_dividends: Usd = ri.div_1099.iter().map(|d| d.box1a_ordinary).sum();
            let qualified_dividends: Usd = ri.div_1099.iter().map(|d| d.box1b_qualified).sum();
            let ceiling = (interest + ordinary_dividends - qualified_dividends).max(Usd::ZERO);
            if investment_interest > ceiling {
                return refusal(
                    RefuseReason::Form4952Required,
                    "you answered that you are NOT filing Form 4952, but your Schedule A line 9 \
                     investment interest is MORE than your investment income from interest and \
                     ordinary dividends minus qualified dividends — the first of the three \
                     conditions of Form 4952's own no-filing exception. Above it, Form 4952 IS \
                     required: §163(d)(1) limits the deduction to your net investment income, a \
                     figure only that form computes, and btctax does not fill it. Deducting the \
                     whole amount on line 9 would UNDERSTATE your tax. File Form 4952 (and this \
                     return) by hand, or correct the line-9 amount. (This return itemizes; a return \
                     that takes the standard deduction is unaffected and computes normally, because \
                     line 9 never prints on it.)",
                );
            }
        }
    }

    // ★★★ §170(f)(8) — THE CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT (P4 / adjudication D4).
    //
    // Two conditions, both required, and BOTH are things liveness cannot see — which is why this sits
    // here and not in the skippable registry:
    //
    //   (i)  the return actually CLAIMS a current-year charitable deduction. `deduction_is_itemized`
    //        is the real §63(e) election (a standard-deduction filer claims no §170 deduction, so
    //        §170(f)(8) conditions nothing for them), and lines 11 + 12 are the §170(b)-limited
    //        amounts actually claimed — a ceiling-zeroed year claims nothing either. ★ Line 13, the
    //        prior-year CARRYOVER, is deliberately EXCLUDED: this year's answer is about THIS year's
    //        gifts, and the carryover year's CWA deadline passed with that year's return (the same
    //        scoping `apply_carryover_writeback` already states).
    //
    //   (ii) at least one SINGLE contribution reaches $250. i1040sca: *"In figuring whether a gift is
    //        $250 or more, don't combine separate donations."* Per contribution, NEVER the year
    //        aggregate — a filer whose every gift is under $250 is never asked. Crypto donations come
    //        from the ledger (one `Removal` = one contribution, measured at the FMV contributed);
    //        non-crypto gifts are one `CharitableGift` entry each.
    //
    // ★★ WHY THIS REFUSES RATHER THAN ADVISES, and the line that stops it generalising:
    //    §170(f)(8)(C) makes an acknowledgment contemporaneous only if obtained by the EARLIER of
    //    filing or the due date, so **filing itself extinguishes the cure**. Refuse where filing
    //    extinguishes the cure; advise where the record can be assembled later — which is why
    //    §170(f)(17) bank-record substantiation for small cash gifts is NOT gated here by analogy.
    let cwa_claimed = ar.schedule_a.as_ref().map_or(Usd::ZERO, |a| {
        a.charitable_cash_11 + a.charitable_noncash_12
    });
    // ★★★ A CEILING-ZEROED YEAR STILL NEEDS THE ACKNOWLEDGMENT (phase-2 review R3, Important).
    //
    // `cwa_claimed > 0` alone treated a §170(b)-ceiling-zeroed year as "nothing claimed, nothing to
    // disallow". That conflates TWO ZEROS THAT DIFFER IN KIND:
    //   - a §170(e)-reduced claim of $0 is EXTINGUISHED, forever. Nothing is ever deducted, so no
    //     acknowledgment is needed. The per-item `claimed_deduction > 0` filter in
    //     `max_single_donation_contribution` handles that one, correctly — it is D4's named misfire.
    //   - a §170(b)-CEILING zero is DEFERRED, not denied. §170(d) carries the claim into the next
    //     five years, this very function's `charitable_carryover_out` computes it, and P6 tells the
    //     filer to roll it forward. The deduction WILL be taken — just later.
    //
    // And §170(f)(8)(C) fixes the acknowledgment deadline at the EARLIER of filing or the due date
    // **of the contribution year**. So the cure dies at THIS filing while the claim lives on. Without
    // this disjunct the low-AGI itemizer with a large crypto gift files unasked, loses the cure
    // permanently (*Durden*), and then deducts across later years unsubstantiated — D4's prong (3),
    // the silently lost right, reappearing on the gate built to close it.
    //
    // Scoped to THIS year's vintage: an older vintage's deadline passed with ITS return, which is the
    // same scoping the line-13 exclusion above already applies.
    let cwa_deferred_to_carryover: Usd = ar
        .charitable_carryover_out
        .iter()
        .filter(|c| c.origin_year == year)
        .map(|c| c.amount)
        .sum();
    if ar.deduction_is_itemized
        && (cwa_claimed > Usd::ZERO || cwa_deferred_to_carryover > Usd::ZERO)
        && a_single_gift_reaches_the_cwa_threshold(ri, state, year)
    {
        // ★★★ WHICH CLAIM THIS RETURN ACTUALLY MAKES (final whole-branch review, finding 2).
        //
        // "this return claims a charitable deduction" is FALSE on the §170(b)-ceiling-zeroed year the
        // R3 fold added to this gate: lines 11 and 12 are $0 there and the whole claim is deferred.
        // A refusal that opens with a statement the filer can see is untrue about their own return
        // teaches them to distrust the rest of it — and the two arms below both depend on the filer
        // believing the premise, because the `Some(false)` cure differs between the cases.
        let what_is_claimed = if cwa_claimed > Usd::ZERO {
            "this return claims a charitable deduction"
        } else {
            "this return's charitable gifts exceeded their §170(b) percentage-of-income ceiling, so \
             none of the deduction is claimed THIS year — but §170(d)(1) carries it forward and you \
             will deduct it in a later year"
        };
        match ri.charitable_cwa_obtained {
            None => {
                return refusal(
                    RefuseReason::CharitableCwaUnresolved,
                    &format!(
                        "{what_is_claimed}, and at least one of your gifts was \
                     $250 or more, so Schedule A lines 11 and 12 send you to the instructions: \
                     \"You can deduct a gift of $250 or more only if you have a contemporaneous \
                     written acknowledgment from the charitable organization.\" §170(f)(8)(A) \
                     makes that a condition of the deduction itself, and btctax has not been told \
                     whether you have one. It will not claim a deduction the statute may deny. \
                     ★ YOU CAN STILL GET ONE — the acknowledgment counts as contemporaneous if \
                     you obtain it \"by the date you file your return or the due date (including \
                     extensions) for filing your return, whichever is earlier\", so ask the \
                     charity now; don't attach it to the return, keep it for your records. \
                     ★★ THE DEADLINE IS THIS RETURN EVEN FOR A CARRIED-FORWARD GIFT: §170(f)(8)(C) \
                     runs from the return for the year of the CONTRIBUTION, not from the year you \
                     finally deduct it, so filing this one without the acknowledgment extinguishes \
                     the cure for every later year too. Run \
                     `btctax income answer`"
                    ),
                );
            }
            Some(false) => {
                // ★ The cure differs by case, and offering the wrong one is worse than offering
                //   none: "remove that gift from the deduction" is meaningless to a filer whose
                //   return deducts nothing this year.
                let cure = if cwa_claimed > Usd::ZERO {
                    "If a charity will not provide one, remove that gift from the deduction"
                } else {
                    "If a charity will not provide one, that gift is not deductible in any year — \
                     §170(f)(8)(A) denies it outright — so it must not be carried forward either; \
                     `--write-carryover` will refuse to persist it"
                };
                return refusal(
                    RefuseReason::CharitableCwaUnresolved,
                    &format!(
                        "you told btctax you do NOT hold a contemporaneous written acknowledgment for \
                     every gift of $250 or more that this return deducts, now or in a later \
                     carryover year. §170(f)(8)(A): \"No \
                     deduction shall be allowed … for any contribution of $250 or more unless the \
                     taxpayer substantiates the contribution by a contemporaneous written \
                     acknowledgment\" — so the deduction as computed is too large, and filing it \
                     would understate your tax. ★ THE CURE IS STILL OPEN, but only until you \
                     file: §170(f)(8)(C) counts an acknowledgment as contemporaneous if you get \
                     it \"by the date you file your return or the due date (including extensions) \
                     for filing your return, whichever is earlier\". Ask each charity for one \
                     showing the amount of money and a description (but not the value) of any \
                     property, and whether it gave you goods or services in return. Then answer \
                     yes and re-run. {cure}"
                    ),
                );
            }
            Some(true) => {}
        }
    }

    // ★★★ **FORM 8960 LINE 9B — §164(b)(6)'s bound on the COLLECTED allocation** (D5).
    //
    // The value is the filer's own: i8960 lets them use "any reasonable method", and choosing one is
    // their election, so btctax collects it and never computes it. What btctax CAN say is how large
    // the pool that method divides is — §1411(c)(1)(B) allocates only "deductions allowed by this
    // subtitle", and SALT above §164(b)(6)(B)'s $10,000/$5,000-MFS cap is not allowed at all.
    //
    // ★ It screens HERE, not in `screen_inputs`, for the same reason the two charitable gates do: the
    //   bound turns on `deduction_is_itemized`, the COMPUTED §63(e) election, which no predicate over
    //   `ReturnInputs` can see. A standard-deduction filer deducted no state income tax, so their
    //   bound is $0 — and that is a real branch, not an edge case.
    //
    // ★★ REFUSES rather than clamping (D5's build-shape guard): shrinking the figure to fit would be
    //    btctax picking the allocation after all, and would rewrite a number signed under §6065.
    if let Some(claimed_9b) = ri.form_8960_line9b {
        let bound = nii_line9b_bound(ar);
        if claimed_9b > bound {
            // ★★★ P3-2 — `salt_cap` IS line 5e, not §164(b)(6)'s ceiling, so it must not be labelled
            //     as the statute's cap. A filer with 5a = $4,000 and 5b = $3,000 (5d = 5e = $7,000)
            //     was told "§164(b)(6)'s $7,000 cap"; the statute's cap is $10,000 and $7,000 is
            //     their own line 5e. The BOUND was right — only the name for it was wrong.
            let salt_5e = ar.schedule_a.as_ref().map_or(Usd::ZERO, |a| a.salt_5e);
            let why = if !ar.deduction_is_itemized {
                "this return takes the STANDARD deduction, so no state or local income tax was \
                 deducted on it at all and none of it is allocable"
                    .to_string()
            } else if ar.schedule_a.as_ref().is_some_and(|a| a.salt_is_sales_tax) {
                "you made the §164(b)(5) election to deduct general SALES taxes instead of income \
                 taxes, and the Instructions for Form 8960 are express: \"Sales taxes aren't \
                 deductible in computing net investment income\" — so the allocable amount is $0"
                    .to_string()
            } else {
                format!(
                    "your Schedule A line 5e — the state and local taxes this return actually \
                     deducted after applying §164(b)(6)'s $10,000 ($5,000 if married filing \
                     separately) limit — is {}, and the state and local INCOME-tax portion of it, \
                     which is all §1411 can allocate, is {}",
                    crate::tax::advisories::fmt_usd(salt_5e),
                    crate::tax::advisories::fmt_usd(bound)
                )
            };
            return refusal(
                RefuseReason::Nii9bExceedsDeductedSalt,
                &format!(
                    "Form 8960 line 9b claims {} of state and local income tax allocable to net \
                     investment income, but {why}. §1411(c)(1)(B) reduces net investment income only \
                     by \"the deductions allowed by this subtitle which are properly allocable\", and \
                     §164(b)(6)(B) limits the state and local tax \"taken into account\" to $10,000 \
                     ($5,000 if married filing separately) — a dollar above that cap is not a \
                     deduction at all, so there is nothing for §1411 to allocate. The Instructions \
                     for Form 8960 agree: the allocable item is state and local income tax \"if \
                     properly deducted on your return when calculating your U.S. regular income \
                     tax\". btctax will not shrink the figure for you, because choosing the \
                     allocation is YOUR election (\"any reasonable method\") and it must not answer \
                     it. Enter a line 9b of {} or less — one reasonable method the instructions \
                     themselves give is that amount times the ratio of Form 8960 line 8 to your AGI \
                     — or clear it to leave line 9b blank",
                    crate::tax::advisories::fmt_usd(claimed_9b),
                    crate::tax::advisories::fmt_usd(bound),
                ),
            );
        }
    }

    // (a) §199A — the sub-schedules of Form 8995-A that btctax does not fill, and the two answers it
    //     needs before it can choose between Form 8995 and Form 8995-A at all.
    //
    // ★★★ §G-28/B1b REPLACED a single blanket `QbiAboveThreshold` here. The old refusal covered every
    //     filer above the §199A(e)(2) threshold on the grounds that "the 8995-A phase-in is unmodeled";
    //     Parts I–III now ARE modelled, so what is left is the sub-schedules of 8995-A that btctax
    //     does not fill: **A** (an SSTB inside the phase-in range), **C** (a prior-year qualified
    //     business loss carryforward) and **D** (a cooperative patron), each with its own named
    //     refusal below. ★ Schedule **B** (aggregation) needs none and gets none: aggregation is an
    //     ELECTION over two or more trades or businesses, btctax has exactly one, so there is nothing
    //     to aggregate and column (c) is correctly unchecked rather than unasked. Keeping one broad reason would have hidden
    //     which schedule was actually missing, and refused the majority of filers who need none of them.
    let reit_dividends: Usd = ri.div_1099.iter().map(|d| d.box5_section_199a).sum();
    // ★★ TRANSCRIBED, not re-derived: `assemble_absolute` already computed this (with the
    //    Schedule 1-A line 13b term, and rounded), and a screen that classifies the regime differently
    //    from the assembler refuses returns the packet would have filed — or files ones it would have
    //    refused.
    let ti_before_qbi = ar.printed_inputs.ti_before_qbi; // Form 8995 line 11 / 8995-A line 20
    let regime = Qbi199aRegime::of(ti_before_qbi, ri.filing_status, params);
    let files_a_199a_form = has_qbi(
        ar.printed_inputs.business_qbi,
        reit_dividends,
        ri.qbi.reit_ptp_carryforward_in,
        ri.qbi.qbi_carryforward_in,
    );
    if files_a_199a_form {
        if let Some(c) = ri.schedule_c.as_ref() {
            // ★★ THE PATRON QUESTION IS MANDATORY AT ANY INCOME, and that is the whole point of it:
            //    Form 8995-A's header says to use that form if taxable income is above the threshold
            //    "or you're a patron of an agricultural or horticultural cooperative", and Form 8995's
            //    "Who Must File" says the same in reverse. An unasked `no` therefore does not merely
            //    leave a box blank — it prints the WRONG FORM.
            match c.is_cooperative_patron {
                None => {
                    return refusal(
                        RefuseReason::CooperativePatronUnanswered,
                        "whether you are a PATRON of an agricultural or horticultural cooperative \
                         decides which §199A form you file: Form 8995-A's own header sends a patron \
                         to that form at ANY income, and Form 8995 excludes them. btctax cannot pick \
                         the form without the answer — run `btctax income answer`",
                    );
                }
                Some(true) => {
                    return refusal(
                        RefuseReason::CooperativePatron,
                        "a patron of an agricultural or horticultural cooperative must reduce the \
                         qualified business income component by an amount figured on Schedule D \
                         (Form 8995-A) — Form 8995-A line 14. btctax does not fill that schedule, and \
                         filing with line 14 blank would OVERSTATE your deduction",
                    );
                }
                Some(false) => {}
            }
            // ★★ The SSTB answer is mandatory only ABOVE the threshold. Below it the simplified Form
            //    8995 has no such checkbox, so the answer changes nothing and demanding it would be a
            //    refusal with no purpose.
            //
            //    ★ Asked BEFORE the sub-schedule refusals below, because if the business is an SSTB
            //      above the range the deduction is zero regardless of wages or UBIA, and demanding
            //      two figures that cannot matter is the wrong question.
            if regime != Qbi199aRegime::AtOrBelowThreshold {
                match c.is_sstb {
                    None => {
                        return refusal(
                            RefuseReason::SstbUnanswered,
                            "above the §199A(e)(2) threshold, whether the business is a SPECIFIED \
                             SERVICE trade or business decides the deduction — past the phase-in \
                             range an SSTB's qualified business income is EXCLUDED ENTIRELY \
                             (§199A(d)(3)), so an unasked `no` would hand you a deduction the statute \
                             denies and understate your tax. It is a checkbox on Form 8995-A because \
                             only you can answer it — run `btctax income answer`",
                        );
                    }
                    // ★★★ INSIDE the range an SSTB is only PARTLY excluded — an "applicable
                    //     percentage" scales its QBI, W-2 wages and UBIA on Schedule A (Form 8995-A)
                    //     before Part I ever runs. ABOVE the range no schedule is needed: the business
                    //     is simply not a qualified trade or business, which core handles.
                    Some(true) if regime == Qbi199aRegime::InPhaseInRange => {
                        return refusal(
                            RefuseReason::SstbInPhaseInRange,
                            "your taxable income is inside the §199A phase-in range and the business \
                             is a SPECIFIED SERVICE trade or business, so only an APPLICABLE \
                             PERCENTAGE of it is treated as a qualified trade or business — figured \
                             on Schedule A (Form 8995-A), which btctax does not fill. Above the range \
                             no such schedule is needed and btctax files the return",
                        );
                    }
                    Some(_) => {}
                }
            }
            // ★★★ THE §199A(b)(2) LIMITATION AMOUNTS. Above the threshold Form 8995-A lines 4 and 7
            //     decide the cap, and `assemble_absolute` defaults both to ZERO when building Parts
            //     I–III — a default that is safe ONLY because this refusal stands in front of it.
            //
            //     Neither direction of a default is safe: zero CAPS the deduction at zero for a filer
            //     who does pay wages (overstating their tax), and anything else invents wages they
            //     never reported (understating it). So `None` refuses at the point of need.
            //
            //     ★ Gated on there BEING a qualified trade or business: an excluded SSTB and a
            //       REIT/PTP-only filer both reach Part IV without ever touching lines 4 or 7, and
            //       demanding two figures nothing reads would be a refusal with no purpose.
            if regime != Qbi199aRegime::AtOrBelowThreshold
                && ar.printed_inputs.business_qbi > Usd::ZERO
                && (c.qbi_w2_wages.is_none() || c.qbi_ubia.is_none())
            {
                return refusal(
                    RefuseReason::QbiAboveThreshold,
                    // ★★ The exit is `income import`, NOT `income answer`. `answer` walks the
                    //    DECLARATION and SKIPPABLE registries — bools and dates — and will never ask
                    //    for a money amount, so sending the filer there is a dead end. Same precedent
                    //    as the Schedule B line 7b country-names refusal above.
                    "above the §199A(e)(2) threshold the deduction is capped by the GREATER of 50% of \
                     the W-2 wages your business paid, or 25% of those wages plus 2.5% of the \
                     unadjusted basis of its qualified property (Form 8995-A lines 4 and 7). btctax \
                     will not guess either one — a guessed zero would cap your deduction at zero, and \
                     any other guess would invent wages you never reported. If your business has no \
                     employees and no property, enter 0 for both and the return files: add \
                     `qbi_w2_wages` and `qbi_ubia` under `[schedule_c]` in the TOML and re-run \
                     `btctax income import`, or fill the \"§199A limitation\" section in the tax-inputs \
                     editor",
                );
            }
        }
        // ★★ A prior-year qualified business net loss carryforward needs Schedule C (Form 8995-A)
        //    before Part I — but ONLY on the 8995-A path. The simplified Form 8995 carries the same
        //    carryforward on its own line 3, which btctax already fills, so this must not refuse below
        //    the threshold. (A CURRENT-year loss refuses further upstream as `ScheduleCLoss`.)
        if regime != Qbi199aRegime::AtOrBelowThreshold && ri.qbi.qbi_carryforward_in > Usd::ZERO {
            return refusal(
                RefuseReason::QbiCarryforwardNeedsSchedule8995AC,
                "a qualified business net loss carried forward from a prior year must be netted on \
                 Schedule C (Form 8995-A) before Form 8995-A Part I — i8995a requires it whenever such \
                 a carryforward exists — and btctax does not fill that schedule. Below the §199A(e)(2) \
                 threshold the simplified Form 8995 carries the same figure on its line 3, and btctax \
                 files that return",
            );
        }
    }

    // (b) Form 6251 Who Must File, condition 1 (§4.11).
    //
    // ★ UNCONDITIONAL — read the real form, not the screening worksheet (whole-branch review I-1).
    // `compute_6251` already ran for every return at the top of `assemble_absolute`, so `ar.amt` is the
    // actual Form 6251. Testing it directly is both cheaper and stronger than testing a proxy:
    //   - The 1040 worksheet only ever concludes "fill in Form 6251" — it is not, and never was, an
    //     authority on whether the form must be ATTACHED. i6251 p.1 is.
    //   - Nesting this gate inside the screen made the branch's own line-2 fix a net SAFETY REDUCTION:
    //     a more-correct screen clears more returns, and clearing meant skipping this check. The gate's
    //     correctness must not depend on a proxy's.
    // The screen survives as a cross-checked soundness claim, not as control flow —
    // `amt_should_file_6251` is now held by `a_cleared_screen_never_hides_a_must_attach_return` below,
    // which asserts the implication `screen clears ⇒ !must_attach` the `amt.rs` module doc argues.
    //
    // Lines 2k, 2l and 3 are $0 in `compute_6251`, which is sound because the three §3 declarations
    // guarantee it: each refuses when UNANSWERED (the registry loop in `screen_inputs`) and when
    // answered ADVERSELY (the value-refusals there), so a return that reaches here has declared all
    // three add-backs inapplicable.
    //
    // Condition 1 (i6251 p.1): "Form 6251, line 7, is greater than line 10." NOT `amt > 0` — when
    // line 7 exceeds line 10 the AMTFTC is figured, so line 9 can still land at or below line 10 and
    // the AMT be $0 while the form is still required.
    // ★★★ §G-6 — THIS REFUSAL IS GONE. btctax computed Form 6251 for every return since v0.14.0 and
    //     could not FILE it, so i6251's Who Must File condition 1 — *"Form 6251, line 7, is greater
    //     than line 10"* — refused the return instead of attaching the form. `f6251.map.toml` and
    //     `btctax-forms::form6251` now emit it at Attachment Sequence 32, and `PrintedForms::f6251`
    //     carries it exactly when `must_attach()` holds.
    //
    //     ★★ WHAT REPLACED IT IS NOT NOTHING. Core models Part I lines 1, 2a, 2b, 3 and 4 only; the
    //     eighteen add-backs 2c-2t have no field, and every one of them is an ADD-BACK — so a filer
    //     who has one and files anyway UNDERSTATES tax. They are censused `gap`, and the gate is the
    //     §G-22 out-of-scope declaration, whose limb (b) names an ISO exercise and limb (c) the other
    //     AMT items. A `yes` there still refuses. That gate had to be widened in the same programme
    //     because an ISO exercise is not INCOME for the regular tax, so the income half of the
    //     question could never have caught it — and because the missing add-back also suppressed this
    //     very `must_attach()` test, the gap hid its own detection (FOLLOWUPS §G-6).

    // ★★★ (c) — THE TI≤0-WITH-A-CARRYFORWARD-IN REFUSAL WAS HERE, AND IT IS GONE (owner-authorised).
    //
    //     It read "btctax models the §1211/§1212 Capital Loss Carryover Worksheet but has not yet
    //     validated FILING this edge". N1 modelled the worksheet; the remaining objection was never
    //     arithmetic but a DECISION — letting a wiped-out year with a loss brought in emit a 1040 and
    //     a Schedule D widens the filing surface on a return signed under §6065. The owner took that
    //     decision, so the screen is deleted rather than kept as a dead variant: a gate whose
    //     off-state is invisible is worse than no gate, and DELETING its `RefuseReason` variant is
    //     what enumerated every consumer for free (E0599 across three crates). The variant's own
    //     identifier is deliberately not written anywhere in `crates/` — including here — because
    //     `xtask`'s `the_lifted_refusal_leaves_no_trace_in_the_tree` asserts exactly that, and a
    //     surviving mention is indistinguishable from a surviving consumer to a grep.
    //
    // ★★ **NO PRINTED LINE MOVES.** This screen ran entirely after `assemble_absolute` and only
    //    decided emission; deleting it changes what is EMITTED, never what is COMPUTED. If any filed
    //    figure changes, the lift was implemented wrongly — e.g. by relaxing the `max(ZERO)` floor on
    //    line 15 instead of deleting a screen — and `the_lift_moves_no_printed_line` exists to catch
    //    exactly that.
    //
    // ★ It was the LAST check in this function, so nothing below it was shadowed and nothing above it
    //   moved.
    None
}

/// Whether at least ONE SINGLE contribution of `year` reaches §170(f)(8)(A)'s $250 threshold.
///
/// i1040sca: *"In figuring whether a gift is $250 or more, don't combine separate donations."* Per
/// contribution, NEVER the year aggregate. Crypto donations come from the ledger (one `Removal` = one
/// contribution, measured at the FMV contributed); non-crypto gifts are one `CharitableGift` each.
///
/// ★ Factored out of the P4 gate so the gate and [`cwa_unvouched_carryover`] cannot drift apart. They
/// answer the same statutory question about the same year's gifts, and a threshold that meant one
/// thing at the refusal and another at the write-back would be a laundering route in itself.
pub fn a_single_gift_reaches_the_cwa_threshold(
    ri: &ReturnInputs,
    state: &LedgerState,
    year: i32,
) -> bool {
    let largest_gift = crate::forms::max_single_donation_contribution(state, year).max(
        ri.schedule_a
            .as_ref()
            .and_then(|a| a.charitable.iter().map(|g| g.amount).max())
            .unwrap_or(Usd::ZERO),
    );
    largest_gift >= crate::tax::tables::CWA_SUBSTANTIATION_THRESHOLD
}

/// ★★★ **THE STANDARD-DEDUCTION DEFERRAL DONOR** (final whole-branch review, finding 1).
///
/// The amount of THIS year's §170(d)(1) carryover that btctax cannot vouch for under §170(f)(8) —
/// `None` when there is nothing unvouched-for.
///
/// **THE SEAM.** Phase 2's R2 fold put `ar.deduction_is_itemized` in front of the §170(f)(8) refusal
/// ("standard deduction ⇒ nothing sworn ⇒ safe"). Phase 2's R3 fold then added the
/// `cwa_deferred_to_carryover` disjunct ("a §170(b)-ceiling zero is DEFERRED, not denied, and the
/// cure dies at THIS filing"). Both are right; they were folded into one un-reviewed commit, and R3's
/// disjunct landed BEHIND R2's conjunct. But **R3's reasoning does not depend on the election**:
/// §170(d)(1) creates the carryover regardless of §63(e) — `apply_170b` runs unconditionally for
/// exactly that reason — so the two rationales collide on the population where the election is
/// standard AND the gift defers. Reproduced: Single renter, AGI $40,000, one $25,000 appreciated-BTC
/// gift ⇒ `deduction_is_itemized` false, `charitable_carryover_out` = $13,000 of CapGainProp30 with
/// `origin_year` 2024, and BOTH screens return `None`. The filer is never asked.
///
/// ★★ **AND THIS IS DELIBERATELY NOT A REFUSAL.** The return that filer prints is CORRECT: it claims
/// no §170 deduction, so §170(f)(8)(A) — which conditions *"a deduction"* — denies nothing on it, and
/// refusing it would be btctax refusing to file a correct return. Worse, the `Some(false)` refusal's
/// cure is *"remove that gift from the deduction"*, which is incoherent for a filer with no deduction
/// to remove — a refusal whose cure its own population cannot perform is finding 2's defect, and
/// hard-refusing here would have recreated it one finding over.
///
/// What IS wrong is what btctax does with the number afterwards, and both halves are fixed:
///
///   1. **It told the filer the carryover was money in the bank** — *"deduction you have already paid
///      for; it is lost if it is never claimed"* — without ever mentioning that §170(f)(8)(A) may
///      have denied it outright. The P6 roll-forward block now says so, and says it BEFORE filing,
///      while §170(f)(8)(C)'s cure is still alive. That is the "asked" half.
///   2. **`--write-carryover` would stamp it `Computed` into next year's inputs**, past every gate:
///      next year's line-13 claim is deliberately outside the P4 gate ("the carryover year's CWA
///      deadline passed with that year's return"), so nothing downstream ever looks again. That is
///      the laundering class the I-2 restriction gate in `apply_carryover_writeback` already guards
///      against, arriving by a second route — and it is refused there now, on the same reasoning.
///
/// Scoped to THIS year's vintage, like the gate: an older vintage's deadline passed with ITS return.
pub fn cwa_unvouched_carryover(
    ri: &ReturnInputs,
    ar: &AbsoluteReturn,
    state: &LedgerState,
    year: i32,
) -> Option<Usd> {
    // `Some(true)` is the filer's own testimony that they hold the acknowledgment; that is exactly
    // what the gate accepts, and it is what btctax vouches on.
    if ri.charitable_cwa_obtained == Some(true) {
        return None;
    }
    let deferred: Usd = ar
        .charitable_carryover_out
        .iter()
        .filter(|c| c.origin_year == year)
        .map(|c| c.amount)
        .sum();
    if deferred <= Usd::ZERO || !a_single_gift_reaches_the_cwa_threshold(ri, state, year) {
        return None;
    }
    Some(deferred)
}

/// Apply the **§4 R3-M6 carryover write-back**: stamp the absolute return's computed charitable, QBI
/// business-loss, QBI-REIT/PTP and **§1212(b) capital-loss** carryover-OUTs into `next_year`'s (year
/// Y+1's) carryover-IN fields, provenance `Computed`.
///
/// Returns the updated `next_year` to persist, or `Err(message)` when it would silently overwrite a
/// **User**-provenance carryover (from `income import`) and `force` is false — never clobbers a user
/// entry. Every conflict is checked BEFORE any field is written (atomic — a QBI conflict does not
/// leave a half-applied charitable write). A computed (or empty) existing carryover-in is overwritten
/// silently.
///
/// ★★ **The capital-loss limb is the one GATED write**, and it is deliberately not symmetric with the
/// other three: it lands only where [`capital_loss_roll_is_grounded`] holds, because a `Computed`
/// stamp is a claim of knowledge. The write-back's caller reads that same predicate to decide what to
/// tell the filer — see [`capital_loss_roll_is_grounded`] for why one definition rather than two.
///
/// ★ This doc comment was for a while attached to the WRONG function — spliced onto
/// `a_single_gift_reaches_the_cwa_threshold` by two commits landing in the same region, leaving the
/// function that authors the `Computed` stamp undocumented. Found by the pre-merge pass (O-1).
pub fn apply_carryover_writeback(
    ar: &AbsoluteReturn,
    ri: &ReturnInputs,
    state: &LedgerState,
    year: i32,
    mut next_year: ReturnInputs,
    force: bool,
) -> Result<ReturnInputs, String> {
    // ★★★ PRE-MERGE I-2 — DO NOT PERSIST A CARRYOVER BTCTAX CANNOT VOUCH FOR.
    //
    // This is the laundering class the two gates in `write_back_carryover` already guard against
    // ("NEVER persist a carryover derived from a pseudo-tainted OR hard-blocked ledger into year+1's
    // stored inputs"), arriving by a third route — and it was OPENED by r3's own I-3 fix.
    //
    // r3 correctly stopped refusing a STANDARD-DEDUCTION year with a declared donation restriction:
    // that return claims no §170 deduction and attaches no 8283, so the restriction moves no figure
    // **on it**. But `apply_170b` runs unconditionally even then, deliberately, so the carryover ages
    // (Reg §1.170A-10(a)(2)) — and the carryover it rolls out is computed at FULL FAIR MARKET VALUE,
    // the number the filer has just told us is too large (Reg §1.170A-7). Persisting that as next
    // year's `Computed` input puts it beyond every gate: `donations_had_restrictions` is `PerYear`
    // so it is `None` on that row, and the §G-21 screen reads `year_donation_deduction(state, Y+1)`,
    // which is $0 because the gift was made in Y.
    //
    // ★ It lives HERE rather than in the CLI so that a future write-back path cannot omit it — the
    // signature makes the omission fail to compile.
    //
    // ★★ `force` does NOT open this. That flag exists to overwrite a figure the USER entered; it is
    // not a licence to write one btctax knows is wrong.
    if !ar.charitable_carryover_out.is_empty() {
        let donated = crate::forms::year_donation_deduction(state, year);
        // ★ FOLD-REVIEW Minor — scoped by `donated`, like its `is_none()` sibling below. Without it
        //   a year with NO donation at all but a rolled-in `charitable_carryover_in` was refused by a
        //   message stating as fact that btctax had valued a donation at full FMV. This year's answer
        //   is about THIS year's gifts; a prior year's carryover is that year's business.
        if ri.donations_had_restrictions == Some(true) && donated > Usd::ZERO {
            return Err(format!(
                "carryover write-back REFUSED for {year}: you declared that a donated property carried \
                 a restriction or a retained right (Form 8283 line 5a/5b/5c), which REDUCES or DENIES \
                 the §170 deduction (Reg §1.170A-7) — but btctax values every donation at full fair \
                 market value, so the ${:.2} carryover it computed is too large. Writing it into \
                 {next}'s inputs would put an inflated figure beyond every check, because next year \
                 has no way to know the gift was restricted. Work the carryover out by hand. \
                 ★ NOTE: this refuses the WHOLE write-back, so your QBI, REIT/PTP and \
                 CAPITAL-LOSS carryforwards were not written either — nothing was persisted. Enter \
                 all four on {next}'s row by hand (`btctax income import`).",
                ar.charitable_carryover_out
                    .iter()
                    .map(|c| c.amount)
                    .sum::<Usd>(),
                next = year + 1
            ));
        }
        if ri.donations_had_restrictions.is_none()
            && donated > crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD
        {
            return Err(format!(
                "carryover write-back REFUSED for {year}: this year's donations file a Form 8283 \
                 SECTION B, whose lines 5a, 5b and 5c ask whether any donated property carried a \
                 restriction — and the answer is not on file. The carryover btctax computed assumes \
                 full fair market value, so it cannot be persisted as {next}'s input without it. \
                 ★ This refuses the WHOLE write-back — your QBI, REIT/PTP and CAPITAL-LOSS \
                 carryforwards were not written either. Run `btctax income answer`, then re-run this \
                 and all four land.",
                next = year + 1
            ));
        }
    }
    // ★★★ FINAL-REVIEW FINDING 1 — THE SAME LAUNDERING CLASS, ARRIVING BY §170(f)(8).
    //
    // The gate above refuses to persist a carryover btctax cannot vouch for because the property was
    // RESTRICTED. This one refuses to persist a carryover it cannot vouch for because the
    // CONTRIBUTION MAY NOT BE DEDUCTIBLE AT ALL: §170(f)(8)(A) is *"No deduction shall be allowed …
    // unless the taxpayer substantiates the contribution by a contemporaneous written
    // acknowledgment"*, and a contribution the statute disallows carries nothing forward under
    // §170(d)(1) either — there is no excess-over-ceiling of a deduction that does not exist.
    //
    // ★ It only ever bites the STANDARD-DEDUCTION deferral donor in practice: an itemizer reaching
    //   this point already answered the P4 gate `Some(true)`, because `None` and `Some(false)` refuse
    //   at `screen_absolute` and never produce a filed return to write back from. That asymmetry is
    //   the finding — see `cwa_unvouched_carryover` for why it is fixed HERE and not by widening the
    //   refusal.
    //
    // ★★ Like its sibling, `force` does NOT open this. That flag overwrites a figure the USER
    //    entered; it is not a licence to write one btctax knows may be disallowed.
    if let Some(unvouched) = cwa_unvouched_carryover(ri, ar, state, year) {
        return Err(format!(
            "carryover write-back REFUSED for {year}: btctax computed a ${unvouched:.2} §170(d)(1) \
             charitable carryover from {year} gifts, but it has not been told you hold a \
             contemporaneous written acknowledgment for the gift of $250 or more behind it. \
             §170(f)(8)(A): \"No deduction shall be allowed … for any contribution of $250 or more \
             unless the taxpayer substantiates the contribution by a contemporaneous written \
             acknowledgment\" — a contribution the statute disallows outright carries nothing into \
             {next} either. Writing it into {next}'s inputs would put it beyond every check: it \
             would be stamped `Computed`, and {next}'s Schedule A line 13 is deliberately outside \
             the acknowledgment gate, because a carryover year's deadline passed with ITS return. \
             ★ This year's return itself was fine to file — it takes the STANDARD deduction and \
             claims no §170 deduction at all, which is why nothing refused at filing time. It is the \
             CARRYOVER that needs the acknowledgment. Run `btctax income answer`; if you hold one, \
             answer yes and re-run and all four carryovers land. If you do not, the lawful outcome \
             is that this carryover does not exist — do not enter it by hand. \
             ★ NOTE: this refuses the WHOLE write-back, so your QBI, REIT/PTP and CAPITAL-LOSS \
             carryforwards were not written either; nothing was persisted.",
            next = year + 1
        ));
    }
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
        // ★ Form 8995 line 16, the same guard: a user-entered business-loss carryforward is the
        // filer's own figure and must not be silently overwritten by ours.
        if next_year.qbi.qbi_carryforward_in > Usd::ZERO
            && next_year.qbi.qbi_carryforward_in_provenance == CarryProvenance::User
        {
            return Err(
                "next year's QBI business-loss carryforward was user-entered — pass `--force` to \
                 overwrite"
                    .to_string(),
            );
        }
        // ★★★ THE FOURTH GUARD — Schedule D lines 6 and 14, and it must PRECEDE every write below.
        //
        // The filer's own carryover is TESTIMONY: it lands on next year's Schedule D lines 6 and 14,
        // which they sign under §6065. Overwriting it silently would put btctax's figure on a line
        // the filer had already sworn to, with nothing to show it had changed.
        //
        // ★ VALUE-CONDITIONED, exactly like the other three, and that is load-bearing rather than
        //   stylistic: a bare `provenance == User` arm would refuse every write into a FRESH next-year
        //   row — `User` is the `CarryProvenance` default, so an untouched `{0,0}` row carries it —
        //   and `writeback_into_fresh_next_year` reds. A zero the filer never entered is not their
        //   testimony.
        if (next_year.capital_loss_carryforward_in.short > Usd::ZERO
            || next_year.capital_loss_carryforward_in.long > Usd::ZERO)
            && next_year.capital_loss_carryforward_in_provenance == CarryProvenance::User
        {
            return Err(
                "next year's capital-loss carryover was user-entered (`income import`) — pass \
                 `--force` to overwrite it with the computed §1212(b) carryover"
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
    next_year.qbi.qbi_carryforward_in = ar.qbi_carryforward_out;
    next_year.qbi.qbi_carryforward_in_provenance = CarryProvenance::Computed;
    // ★★★ **THE §1212(b) CARRYOVER, ROLLED — r3 I-4 ANSWERED, NOT REVERSED.**
    //
    // r3 I-4 removed a stamp of `capital_loss_carryforward_in_provenance = Computed` on a value the
    // write-back never wrote. Its reasoning — *"a provenance stamp is a CLAIM OF KNOWLEDGE; do not
    // make one the code cannot support"* — is unchanged and still governs every line below. What
    // moved is its PREMISE: N1 modelled the worksheet, so `AbsoluteReturn::
    // capital_loss_carryforward_out` exists and there IS a value to write.
    //
    // So the stamp is founded only where the figure descends from something btctax actually knows,
    // and groundedness is CHECKED rather than assumed. The three grounds, the one excluded case, and
    // the whole-dollar rounding each have exactly ONE home — [`capital_loss_roll_is_grounded`] and
    // [`rounded_capital_loss_carryforward_out`] below. They are functions rather than inline code
    // because the write-back's SUMMARY has to ask the same question, and a second copy of either
    // would be the widening review's B-1 waiting to happen again.
    if capital_loss_roll_is_grounded(ar, ri) {
        next_year.capital_loss_carryforward_in = rounded_capital_loss_carryforward_out(ar);
        next_year.capital_loss_carryforward_in_provenance = CarryProvenance::Computed;
    }
    // ★ §G-20a — the CHARITABLE carryover gets its provenance stamped too. Without this a computed
    // ZERO stays indistinguishable from an unasked one, and next year's advisory nags a filer whose
    // prior year btctax itself computed.
    next_year.charitable_carryover_in_provenance = CarryProvenance::Computed;
    Ok(next_year)
}

/// The §1212(b) carryover-OUT as [`apply_carryover_writeback`] would PERSIST it.
///
/// ★★ ROUNDED TO WHOLE DOLLARS, deliberately. The persisted figure becomes next year's Schedule D
///    lines 6 and 14 — lines the filer READS OFF THE PAGE and swears to. The measured H9 vector is
///    $42,871.66 exact against $42,872 hand-worked off the filed page; rounding here ties the stored
///    value to the page. (Residual, accepted: the printed Schedule D re-derives lines 7/15/16 from
///    per-row-rounded Form 8949 cells while the worksheet reads exact `CapNet`, so a reader
///    hand-working the page can still land ~$1/row off. Closing that means re-sourcing the worksheet
///    from the printed chain — a layering change, not this one.)
pub fn rounded_capital_loss_carryforward_out(ar: &AbsoluteReturn) -> Carryforward {
    Carryforward {
        short: round_dollar(ar.capital_loss_carryforward_out.short),
        long: round_dollar(ar.capital_loss_carryforward_out.long),
    }
}

/// ★★★ **Whether [`apply_carryover_writeback`] can VOUCH for a §1212(b) capital-loss roll.**
///
/// A provenance stamp is a CLAIM OF KNOWLEDGE (r3 I-4), so the roll happens only where the figure
/// descends from something btctax actually knows. Three grounds, any one of which suffices:
///   * year Y's carryover-in was itself `Computed`   → the inductive step;
///   * year Y's carryover-in is a nonzero `User` one → the filer's own testimony, a real base case;
///   * year Y produced a nonzero carryover-OUT       → btctax computed this year's loss itself.
///
/// ★★★ THE ONE EXCLUDED CASE is a year that was never asked AND produced nothing. Writing `{0,0}` +
/// `Computed` there would silence next year's `BenefitCarryoversNotStated` about a carryover the
/// filer may genuinely have — which is verbatim the r3 I-4 damage. It stays closed, and no VALUE
/// assertion can see it: the stored amount is zero either way.
///
/// ★★ **ONE DEFINITION, TWO READERS — and that is exactly why it is a function.** The widening review
/// found the write-back's own SUMMARY claiming this write on the branch where the gate skipped it:
/// the summary read the row FIELD (which on an ungrounded roll is either an untouched `{0,0}` or a
/// figure an EARLIER roll stamped) instead of asking whether anything had been assigned. A summary
/// that re-derives the predicate inline would be the same defect one edit away, so the writer and the
/// message it prints now read the same predicate. Reproduced before it was fixed; see
/// `the_summary_does_not_claim_a_capital_loss_write_the_gate_skipped`.
pub fn capital_loss_roll_is_grounded(ar: &AbsoluteReturn, ri: &ReturnInputs) -> bool {
    ri.capital_loss_carryforward_in_provenance == CarryProvenance::Computed
        || ri.capital_loss_carryforward_in != Carryforward::default()
        || rounded_capital_loss_carryforward_out(ar) != Carryforward::default()
}

/// Schedule B §6012 / Form 1040 Schedule B filing threshold ($1,500 for interest and for dividends).
const SCHEDULE_B_THRESHOLD: Usd = dec!(1500);

/// Whether Schedule B must be filed (SPEC §7.1, R3-I2 — the single normative site): **taxable interest >
/// $1,500** OR **ordinary dividends > $1,500** OR a Part III trigger is affirmed —
/// `foreign_accounts == Some(true)` (trigger b) OR `foreign_trust == Some(true)` (trigger, §2.9).
/// Uses the NON-crypto 1040 2b / 3b figures (crypto lending interest lands on Sch 1 L8v, not 2b).
///
/// ★ P9 §2.9: the foreign-account/-trust ANSWERS are collected unconditionally by the FORM_QUESTIONS
/// registry (they are live ALWAYS), so this predicate no longer gates whether they are asked — it only
/// decides whether the schedule PRINTS. Adding `foreign_trust` here is belt-and-braces: the predicate is
/// true whenever Part III is required, independent of screen order. (A `foreign_trust == Some(true)` return
/// refuses upstream as unsupported, so this branch is refusal-shadowed at the print — deliberately kept so
/// the predicate is correct on its own terms.)
pub fn schedule_b_files(ri: &ReturnInputs) -> bool {
    sum_taxable_interest(ri) > SCHEDULE_B_THRESHOLD
        || sum_ordinary_dividends(ri) > SCHEDULE_B_THRESHOLD
        || ri.foreign_accounts == Some(true)
        || ri.foreign_trust == Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::SaltLimitation;
    // `Person` is a TEST-only import now: the §63(f) box count moved to `packet::AgedBlindBoxes`, which
    // is the single source L12 consumes (`p6-aged-blind-checkboxes-missing`).
    use crate::tax::return_inputs::Person;
    // ★ TEST-only since whole-branch review I-1: the AMT screening worksheet no longer appears in any
    // production path — `screen_absolute` reads the real `ar.amt.must_attach()`. The two helpers survive
    // only as the cross-check that `a_cleared_screen_never_hides_a_must_attach_return` exercises.
    use crate::tax::amt::{amt_should_file_6251, amt_worksheet_line2};

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

    // ── §G-21 — the restriction questions, re-keyed by r3 I-2/I-3 ────────────────────────────────

    /// A donation of `claimed`, made in 2024.
    /// A donation of `claimed`, made in 2024, **with a real long-term leg**.
    ///
    /// ★★★ FOLD-REVIEW — THE LEG IS THE POINT. This fixture originally set `claimed_deduction` with
    /// `legs: vec![]`, which is a shape no real ledger produces: `year_donation_deduction` reads
    /// `claimed_deduction` and saw the gift, while `crypto_charitable_gifts` iterates the LEGS and
    /// saw nothing — so Schedule A line 12 was `0` and the return claimed no deduction at all. The
    /// r3 tests therefore passed against the LEDGER figure alone, which is precisely the defect the
    /// fold review found. A test whose fixture cannot reach the return cannot pin a rule about the
    /// return.
    fn donation_state(claimed: Usd) -> LedgerState {
        state_removals(vec![donation(
            date!(2024 - 09 - 09),
            vec![donation_leg(Term::LongTerm, claimed / dec!(5), claimed)],
        )])
    }

    /// ★★★ **r3 I-2 — the gate was TOO NARROW, and the gap was an UNDERSTATEMENT.**
    ///
    /// The refusal was keyed on `year_donation_deduction > $5,000`, the Form 8283 SECTION split. But
    /// §170(f)(11)(C)'s $5,000 only decides which section of the form you file — Reg §1.170A-7 and
    /// §170(f)(3)(A) reduce or deny a restricted gift's deduction at EVERY dollar amount. So a filer
    /// who explicitly declared a restriction on a $4,000 donation had the declaration discarded and
    /// the full fair market value deducted: btctax filing a number it held the filer's own testimony
    /// against.
    ///
    /// Mutation-verified: restoring the `> QUALIFIED_APPRAISAL_THRESHOLD` guard on the `Some(true)`
    /// arm reds the sub-threshold row.
    #[test]
    fn a_declared_restriction_refuses_at_any_amount_not_just_over_5000() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let screened = |claimed: Usd, answer: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                // §170(f)(8) answered NEUTRAL so this test keeps testing the §G-21 RESTRICTION
                // question. The donation here is well over $250, so the CWA gate is genuinely live.
                charitable_cwa_obtained: Some(true),
                // Force the itemized election so the §170 deduction is genuinely claimed.
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    salt_state_estimated_payments: dec!(10000),
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                w2s: vec![w2(
                    Owner::Taxpayer,
                    dec!(200000),
                    dec!(168600),
                    dec!(200000),
                )],
                ..Default::default()
            };
            let st = donation_state(claimed);
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            assert!(ar.deduction_is_itemized, "the fixture must itemize");
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };

        // ★ THE DEFECT: a declared restriction under $5,000 sailed through and deducted full FMV.
        assert_eq!(
            screened(dec!(4000), Some(true)),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "Reg §1.170A-7 has no dollar floor — a restricted gift's deduction is wrong at $4,000 too"
        );
        assert_eq!(
            screened(dec!(9000), Some(true)),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "…and still refuses over the threshold"
        );

        // UNANSWERED is different, and correctly still keyed on $5,000: below it the form never
        // PRINTS 5a/5b/5c, so silence forgoes nothing and asserts nothing.
        assert_eq!(
            screened(dec!(4000), None),
            None,
            "a Section A year never poses the question — do not block a small donor"
        );
        assert_eq!(
            screened(dec!(9000), None),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "a Section B year PRINTS the three boxes — btctax may not answer them for the filer"
        );

        // And an explicit No files at both sizes.
        assert_eq!(screened(dec!(4000), Some(false)), None);
        assert_eq!(screened(dec!(9000), Some(false)), None);
    }

    /// ★★★ **r3 I-3 — the gate was TOO WIDE, and it BLOCKED a correct return.**
    ///
    /// `year_donation_deduction` reads the LEDGER, not the return. A filer who takes the STANDARD
    /// deduction claims no §170 deduction and attaches no Form 8283 at all (`packet.rs`: "a
    /// standard-deduction year with donations files none"), so a restriction changes no figure on
    /// the return — yet the old gate refused them, and the refusal was **unescapable**: the only
    /// exits were answering "No" (perjury) or deleting a real ledger event. Its message also
    /// asserted "this year files a Form 8283 SECTION B", which was simply false.
    ///
    /// Mutation-verified: dropping the `ar.deduction_is_itemized` term reds both rows.
    #[test]
    fn a_standard_deduction_year_is_never_blocked_by_the_restriction_questions() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let screened = |answer: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                // No Schedule A inputs ⇒ itemized ($6,000 of gifts) < standard ($14,600).
                w2s: vec![w2(Owner::Taxpayer, dec!(80000), dec!(80000), dec!(80000))],
                ..Default::default()
            };
            let st = donation_state(dec!(6000));
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            assert!(
                !ar.deduction_is_itemized,
                "the fixture must take the STANDARD deduction"
            );
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };

        for answer in [None, Some(true), Some(false)] {
            assert_eq!(
                screened(answer),
                None,
                "a standard-deduction year claims no §170 deduction and files no 8283 — the \
                 restriction cannot move a figure, so refusing is a false block with no exit \
                 (answer was {answer:?})"
            );
        }
    }

    /// ★ **Fable P6 r1 I6.** The Form 8283 trigger is an AGGREGATE — Schedule A line 12's "over $500" and
    /// the 8283's own "a total deduction of over $500 for **all** contributed property". Keying the
    /// refusal on the user-entered gifts alone let the MIXED case through: $300 of non-crypto noncash
    /// (under the threshold on its own) plus $400 of crypto donations from the ledger ⇒ L12 = $700, so an
    /// 8283 IS required — and the one btctax can build lists only the crypto rows, under-reporting its own
    /// property list and putting the whole deduction at risk (§170(f)(11)).
    #[test]
    fn mixed_noncash_gifts_over_the_aggregate_8283_threshold_refuse() {
        use crate::state::{Removal, RemovalKind, RemovalLeg};
        use crate::tax::return_inputs::{CharitableClass, CharitableGift, ScheduleAInputs};

        let donation = |claimed: Usd| Removal {
            event: EventId::decision(9),
            kind: RemovalKind::Donation,
            removed_at: date!(2024 - 07 - 04),
            legs: Vec::<RemovalLeg>::new(),
            appraisal_required: false,
            donor_acquired_at: None,
            claimed_deduction: Some(claimed),
            donee: Some("Habitat".into()),
        };
        let st = LedgerState {
            removals: vec![donation(dec!(400))], // crypto donations from the LEDGER
            ..Default::default()
        };
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.schedule_a = Some(ScheduleAInputs {
            charitable: vec![CharitableGift {
                class: CharitableClass::CapGainProp30, // non-crypto NONCASH — no 8283 rows exist for it
                amount: dec!(300),                     // under $500 ON ITS OWN…
            }],
            ..Default::default()
        });

        // …but $300 + $400 = $700 of noncash ⇒ Schedule A L12 > $500 ⇒ an 8283 is required.
        assert_eq!(
            screened(&ri, &st),
            Some(RefuseReason::NonCryptoNoncashGift),
            "the aggregate crosses the threshold, so the incomplete 8283 must not be attached"
        );

        // Crypto-only donations over the threshold are FINE — btctax has every row for those.
        let mut crypto_only = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        crypto_only.schedule_a = Some(ScheduleAInputs::default());
        assert_eq!(screened(&crypto_only, &st), None);
    }
    /// A Single household the synthetic table can price (it only carries `Single` schedules). Tuned so
    /// the ordinary base (`ordinary_taxable_income`) sits just below the synthetic $100k bracket edge:
    /// wages 98,600 + int 4,000 + ord-div 10,000 + cap-gain-distr 3,000 = AGI 115,600; taxable 101,000;
    /// ordinary base = 101,000 − 8,000 qd − 3,000 cap-gain = 90,000.
    fn single_household() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            // D-8: an ordinary filer, who has ANSWERED that nobody can claim them.
            header: crate::tax::testonly::not_a_dependent(),
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

    // (`schedule_b_part3_none_is_fail_loud_only_when_filing` DELETED — P9 §2.9. Its premise ("only when
    //  filing") WAS the circular-liveness bug: the foreign questions must be answered on EVERY return,
    //  because their own answer is what decides whether Schedule B files. The correct behaviour is now
    //  tested in return_refuse::tests (`a_foreign_account_question_is_live_even_below_the_schedule_b_threshold`
    //  and the per-question property test), and the `schedule_b_part3_unanswered` predicate is deleted.)

    // ── §63 standard deduction (Phase 3 task 1) ──────────────────────────────────────────────────
    fn person(dob: Option<Date>, blind: bool) -> Person {
        Person {
            date_of_birth: dob,
            blind: Some(blind),
            ..Default::default()
        }
    }
    fn filer(status: FilingStatus) -> ReturnInputs {
        ReturnInputs {
            filing_status: status,
            // D-8: an ordinary filer, who has ANSWERED that nobody can claim them.
            header: crate::tax::testonly::not_a_dependent(),
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
        blind.header.taxpayer.blind = Some(true);
        assert_eq!(standard_deduction(&blind, &p, 2024, Usd::ZERO), dec!(16550));
        // MFJ, BOTH spouses 65+ → basic $29,200 + 2 × $1,550 = $32,300.
        let mut mfj = filer(FilingStatus::Mfj);
        mfj.header.taxpayer.date_of_birth = Some(date!(1955 - 06 - 01));
        mfj.header.spouse = Some(person(Some(date!(1955 - 06 - 01)), false));
        mfj.header.spouse_died_during_year = Some(false); // §G-9 — answered, not defaulted
        assert_eq!(standard_deduction(&mfj, &p, 2024, Usd::ZERO), dec!(32300));
    }

    /// ★★ §G-9 — THE IRS's OWN BOUNDARY PAIR, verbatim from i1040gi (2024, *Standard Deduction*):
    /// *"If your spouse was born before January 2, 1960, but died in 2024 before reaching age 65, don't
    /// check the box that says 'Spouse was born before January 2, 1960.' **A person is considered to
    /// reach age 65 on the day before the person's 65th birthday.**"*
    ///
    /// So for TY2024 a person born 1959-02-14 reaches 65 on **2024-02-13**: dying that day qualifies,
    /// dying the day before does not. The pre-§G-9 code decided the box from the date of BIRTH alone and
    /// granted it on both sides — understating the tax by the value of a $1,950 (unmarried) or $1,550
    /// (married) addition. Neither oracle can catch it: OTS takes a filer-answered `"You_65+Over?"`
    /// boolean and Tax-Calculator has only an `age_spouse` integer.
    #[test]
    fn the_irs_death_boundary_pair_decides_the_aged_box() {
        let dob = Some(date!(1959 - 02 - 14));
        // Born early enough for TY2024 on its own (≤ Jan 1 1960) — this is the fact §G-9 used ALONE.
        assert!(
            is_aged(dob, Some(false), None, 2024),
            "no death ⇒ the DOB test is the whole test"
        );

        // Reached 65 on 2024-02-13, the day BEFORE the 65th birthday.
        assert!(
            is_aged(dob, Some(true), Some(date!(2024 - 02 - 13)), 2024),
            "died ON the day they reached 65 ⇒ qualifies"
        );
        assert!(
            !is_aged(dob, Some(true), Some(date!(2024 - 02 - 12)), 2024),
            "died the day BEFORE reaching 65 ⇒ does NOT qualify (i1040gi)"
        );

        // …and the dollars, through the production standard deduction. $14,600 + $1,950 vs $14,600.
        let p = ty2024_params();
        let mk = |dod| {
            let mut r = filer(FilingStatus::Single);
            r.header.taxpayer.date_of_birth = dob;
            r.header.taxpayer_died_during_year = Some(true);
            r.header.taxpayer.date_of_death = Some(dod);
            r
        };
        assert_eq!(
            standard_deduction(&mk(date!(2024 - 02 - 13)), &p, 2024, Usd::ZERO),
            dec!(16550)
        );
        assert_eq!(
            standard_deduction(&mk(date!(2024 - 02 - 12)), &p, 2024, Usd::ZERO),
            dec!(14600),
            "the §G-9 defect: this used to be 16,550"
        );
    }

    /// ★ §G-9, the two fail-closed arms. A death whose DATE was skipped, and an UNANSWERED death gate,
    /// must both FORGO the addition — never grant it. The skip is lawful (class B: the burden to claim
    /// is the filer's); the unanswered gate is refused by `screen_inputs` before it can reach here, and
    /// this pins the belt-and-braces so no future caller bypassing the screen leaks the defect back in.
    #[test]
    fn a_dateless_or_unanswered_death_forgoes_the_aged_box() {
        let dob = Some(date!(1950 - 01 - 01)); // comfortably over 65, so only the death logic can deny
        assert!(is_aged(dob, Some(false), None, 2024));
        assert!(
            !is_aged(dob, Some(true), None, 2024),
            "died, date skipped ⇒ cannot be shown to have reached 65 ⇒ forgone"
        );
        assert!(
            !is_aged(dob, None, None, 2024),
            "death gate UNANSWERED ⇒ never granted"
        );
    }

    /// ★ §G-9 leap day. Someone born **February 29** has no 65th birthday, so `Date::replace_year`
    /// fails; the convention is that they attain the age on March 1, hence reach 65 on February 28.
    /// Without the fallback in `reaches_65_on` this filer could never qualify at all.
    #[test]
    fn a_leap_day_birth_reaches_65_on_february_28() {
        let dob = date!(1960 - 02 - 29);
        assert_eq!(reaches_65_on(dob), Some(date!(2025 - 02 - 28)));
        assert!(is_aged(
            Some(dob),
            Some(true),
            Some(date!(2025 - 02 - 28)),
            2025
        ));
        assert!(!is_aged(
            Some(dob),
            Some(true),
            Some(date!(2025 - 02 - 27)),
            2025
        ));
        // A non-leap birth one day later is the ordinary path: 65th birthday 2025-03-01, reached 02-28.
        assert_eq!(
            reaches_65_on(date!(1960 - 03 - 01)),
            Some(date!(2025 - 02 - 28))
        );
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
        dep.header.can_be_claimed_as_dependent_taxpayer = Some(true);
        // Earned $0 → max($1,300, $450) = $1,300.
        assert_eq!(standard_deduction(&dep, &p, 2024, Usd::ZERO), dec!(1300));
        // Earned $5,000 → max($1,300, $5,450) = $5,450 (< basic).
        assert_eq!(standard_deduction(&dep, &p, 2024, dec!(5000)), dec!(5450));
        // Earned $20,000 → $20,450 capped at basic $14,600.
        assert_eq!(standard_deduction(&dep, &p, 2024, dec!(20000)), dec!(14600));
        // Dependent + blind → floor base ($1,300) + $1,950 aged/blind.
        let mut db = dep.clone();
        db.header.taxpayer.blind = Some(true);
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

    /// ★ 1040 L14 is a THREE-term sum from 2025, and Form 8995 line 11 excludes only two of them.
    ///
    /// The 2025 form reads L14 = "Add lines 12e, 13a, and **13b**", where 13b is Schedule 1-A's total.
    /// And i8995 figures line 11 — taxable income *before* the QBI deduction — as 1040 line 11a minus
    /// lines **12e and 13b**, deliberately NOT minus 13a, because 13a is the deduction being computed.
    ///
    /// So the two consumers differ, and getting `ti_before_qbi` wrong is directional: omitting 13b
    /// OVERSTATES it, which inflates the §199A deduction and fires `qbi_over_threshold` too EARLY — a
    /// false refusal.
    ///
    /// ★ **This test is DELIBERATELY WEAK, and saying so is the point.** `assemble_absolute` hardcodes
    /// a zero 13b until Schedule 1-A lands (B3), so both assertions below are trivial identities and
    /// every mutation of the composition survives them — verified: dropping the 13b term from
    /// `total_deductions`, and subtracting 13a from `ti_before_qbi`, both leave this green. It records
    /// the SHAPE so B3 has something to make real. The load-bearing version is
    /// `printed::tests::printed_1040_line14_needs_all_three_terms`, which drives the printed path with
    /// a nonzero 13b and does die to that mutation.
    #[test]
    fn form_1040_line14_sums_12e_13a_and_13b_while_form_8995_line11_excludes_only_12e_and_13b() {
        let ri = crate::tax::testonly::answered(ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(120000),
                ..Default::default()
            }],
            ..Default::default()
        });
        let ar = assemble_absolute(
            &ri,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert_eq!(
            ar.total_deductions,
            ar.deduction + ar.qbi_deduction + ar.schedule_1a_additional,
            "L14 = 12e + 13a + 13b"
        );
        // Form 8995 line 11 lives on `PrintedInputs`, carried on the assembled return.
        let pi = &ar.printed_inputs;
        assert_eq!(
            pi.ti_before_qbi,
            ar.agi - ar.deduction - ar.schedule_1a_additional,
            "Form 8995 line 11 = 1040 11a − 12e − 13b; it must NOT subtract 13a (the QBI deduction \
             it is computing) and it must NOT skip 13b (which would overstate it and refuse early)"
        );

        assert_eq!(
            ar.schedule_1a_additional,
            Usd::ZERO,
            "TY2024's form has no line 13b, so zero is the correct value — not a stub"
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

    /// ★ P9 §2.7 — a MIXED-USE mortgage (`mortgage_all_used_to_buy_build_improve == Some(false)`) on a
    /// Schedule A that reports mortgage interest ZEROES line 8a and CHECKS the line-8 box: v1 cannot do the
    /// Pub. 936 allocation, and §163(h)(3)(F) makes the non-acquisition portion non-deductible, so it claims
    /// NONE of it ($0 ≤ the true allocation always ⇒ tax overstated, never understated).
    /// `mixed_use_mortgage_forgone` reports the FULL 1098 interest as the ceiling.
    #[test]
    fn mixed_use_mortgage_zeroes_8a_and_checks_the_box() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(12000),
            mortgage_all_used_to_buy_build_improve: Some(false),
            ..Default::default()
        });
        let parts = schedule_a_parts(&r, dec!(100000), &no_charity(), &ty2024_params()).unwrap();
        assert_eq!(
            parts.mortgage_8a,
            Usd::ZERO,
            "8a zeroed — v1 cannot allocate"
        );
        assert!(parts.mortgage_mixed_use_box, "the line-8 box is checked");
        assert_eq!(
            parts.total_17,
            Usd::ZERO,
            "no other Schedule A items ⇒ total is $0"
        );
        assert_eq!(
            mixed_use_mortgage_forgone(&r),
            Some(dec!(12000)),
            "the forgone ceiling is the full 1098 interest"
        );
    }

    /// `Some(true)` (acquisition-only) keeps line 8a FULL and the box unchecked; and there is nothing
    /// forgone. The value behaviour fires ONLY on an explicit "no".
    #[test]
    fn acquisition_only_mortgage_keeps_full_8a_and_box_off() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: dec!(12000),
            mortgage_all_used_to_buy_build_improve: Some(true),
            ..Default::default()
        });
        let parts = schedule_a_parts(&r, dec!(100000), &no_charity(), &ty2024_params()).unwrap();
        assert_eq!(parts.mortgage_8a, dec!(12000));
        assert!(!parts.mortgage_mixed_use_box);
        assert_eq!(mixed_use_mortgage_forgone(&r), None);
    }

    /// ★ §2.7 third row (r6 Nit-3): the box-check and the forgone amount are BOTH scoped to the LIVE
    /// predicate (interest > 0), never to the bare `Some(false)`. A $0-interest "no" forgoes nothing, so
    /// the box stays unchecked and nothing is forgone.
    #[test]
    fn zero_interest_mixed_use_checks_no_box() {
        let mut r = filer(FilingStatus::Single);
        r.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: Usd::ZERO,
            mortgage_all_used_to_buy_build_improve: Some(false),
            ..Default::default()
        });
        let parts = schedule_a_parts(&r, dec!(100000), &no_charity(), &ty2024_params()).unwrap();
        assert_eq!(parts.mortgage_8a, Usd::ZERO);
        assert!(
            !parts.mortgage_mixed_use_box,
            "no interest ⇒ not live ⇒ box unchecked"
        );
        assert_eq!(mixed_use_mortgage_forgone(&r), None);
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
            salt_use_sales_tax: Some(true),
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
            // D-8: an ordinary filer, who has ANSWERED that nobody can claim them.
            header: crate::tax::testonly::not_a_dependent(),
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
            ri.header.can_be_claimed_as_dependent_taxpayer = Some(true);
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
        not_dep.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        assert_eq!(screened(&not_dep, &empty), None);

        // ★ D-8, fail-closed. An UNANSWERED flag must still RUN this screen, not skip it. `screen_inputs`
        // refuses `None` long before compute, so this is defense-in-depth — but its direction is the whole
        // point: skipping can only UNDER-refuse (a real kiddie return slips through at the child's rate),
        // while running it can only over-refuse. `unwrap_or(false)` here skips. `!= Some(false)` runs.
        let mut unknown = dependent(dec!(9000));
        unknown.header.can_be_claimed_as_dependent_taxpayer = None;
        assert_eq!(
            screened(&unknown, &empty),
            Some(RefuseReason::KiddieTax),
            "an unknown dependent flag must not silently skip the §1(g) screen"
        );
    }

    /// Wages (earned) do NOT count toward the kiddie unearned threshold — a working dependent with big
    /// wages but small investment income is not kiddie-refused.
    #[test]
    fn kiddie_excludes_earned_wages() {
        let mut ri = single();
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(true);
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
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(true);
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
        // ★★★ §G-28/B1a — THIS FILER NOW FILES. TI-before-QBI = 251,000 − 14,600 = 236,400 > 191,950,
        //     but their only §199A item is REIT dividends: there is no trade or business for the
        //     W-2-wage/UBIA limitations or the SSTB phase-in to attach to. i8995a says so — "If you
        //     don't have QBI, and only have REIT, PTP, skip Parts I through III and complete Part IV" —
        //     and Part IV needs no input btctax lacks. Refusing them was refusing a return the form
        //     itself tells us how to complete.
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "a REIT/PTP-only filer above the threshold files on Form 8995-A Part IV"
        );
        // ★★ …and they file 8995-A, NOT the simplified Form 8995. Getting that backwards prints a form
        //    i8995a's "Who Must File" forbids at this income.
        assert!(
            crate::tax::qbi::uses_8995a(
                ar.printed_inputs.business_qbi,
                dec!(1000),
                Usd::ZERO,
                Usd::ZERO,
                ar.agi - ar.deduction,
                FilingStatus::Single,
                &p,
            ),
            "above the threshold the simplified Form 8995 no longer applies"
        );
        // Drop the REIT dividends → no QBI at all → no refuse even at the same high income.
        let mut no_qbi = ri.clone();
        no_qbi.div_1099[0].box5_section_199a = Usd::ZERO;
        let ar2 = assemble_absolute(&no_qbi, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            screen_absolute(&no_qbi, &ar2, &p, &empty_ledger(), 2024),
            None
        );
    }

    /// ★ **Fable P7 r1 I2.** The same §199A(e)(2) refuse, driven by a **Schedule C trade or business**
    /// with NO REIT dividends anywhere.
    ///
    /// The test above only ever exercised the REIT leg (`box5_section_199a`), so the `business_qbi`
    /// argument to `qbi_over_threshold` was load-bearing and untested: the reviewer replaced it with
    /// `Usd::ZERO` — deleting the refusal for every Schedule C filer — and all 1702 tests still passed.
    ///
    /// ★★★ §G-28/B4 r1-I1 — OFFSETTING 1099-B TOTALS STILL FILE A SCHEDULE D.
    ///
    /// `must_file()` never learned about lines 1a/8a. Line 16 is `line7 + line15`, so it is exactly
    /// zero when the short- and long-term totals cancel — and line 16 was the only term in that gate
    /// which could have carried them. A filer with $1,050,000 of short-term proceeds and $400,000 of
    /// long-term proceeds, every dollar reported to the IRS on a Form 1099-B, filed a packet with NO
    /// SCHEDULE D IN IT.
    ///
    /// ★★ NOTE THE SHAPE: no dollar was wrong. 1040 line 7 was correctly zero. The defect is a
    /// REQUIRED SCHEDULE OMITTED, which is invisible to every test that checks a value — so this test
    /// checks the ATTACHMENT DECISION, which is a different question.
    #[test]
    fn offsetting_1099b_totals_still_require_a_schedule_d() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                short_term_proceeds: dec!(1050000),
                short_term_basis: dec!(1000000), // +50,000 short-term
                long_term_proceeds: dec!(400000),
                long_term_basis: dec!(450000), // −50,000 long-term
                basis_reported_and_no_adjustments: Some(true),
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let ar = assemble_absolute(&ri, &crate::state::LedgerState::default(), &p, &table, 2024);
        let pr = crate::tax::packet::assemble_printed_return(
            &ri,
            &crate::state::LedgerState::default(),
            &std::collections::BTreeMap::new(),
            &ar,
            &table,
            2024,
            &[],
        )
        .expect("the printed return assembles");
        let d = &pr.forms.sch_d;
        // The fixture must actually OFFSET, or the old gate would have caught it anyway.
        assert_eq!(
            d.line16,
            Usd::ZERO,
            "the two characters must cancel, else this is vacuous"
        );
        assert_eq!(d.line7, dec!(50000));
        assert_eq!(d.line15, dec!(-50000));
        assert!(
            d.must_file(),
            "$1,450,000 of proceeds were reported to the IRS on Forms 1099-B — the schedule that \
             reports them cannot be omitted just because the two characters happen to net to zero"
        );
    }

    /// ★★ §G-28/B4 — a return with no 1099-B carries NO 1a/8a AMOUNTS into the printed chain.
    ///
    /// Line 1a's own text ends *"However, if you choose to report all these transactions on Form 8949,
    /// **leave this line blank** and go to line 1b."* A printed `0` there is not a neutral zero: it
    /// swears the filer had Form 1099-B transactions, with basis reported to the IRS, totalling
    /// nothing. This is the §G-24 class, and the first draft of B4 reintroduced it for EVERY
    /// pure-crypto return.
    ///
    /// ★★★ RENAMED AFTER r3, AND THE OLD NAME WAS THE FINDING. It was
    /// `..._leaves_lines_1a_and_8a_blank`, which claims the blank-vs-zero distinction — and the body
    /// asserts a ZERO, over a fold across an EMPTY vector. It is an arithmetic identity: restoring the
    /// whole "prints a sworn 0" defect left it green. A `0` and a blank are exactly what the old name
    /// said it distinguished and exactly what it could not, which is §G-24's own thesis turned on the
    /// test written to honour it.
    ///
    /// What it legitimately holds is that no AMOUNT reaches the printed chain. The BLANK-ness is held
    /// where blankness exists — `schedule_d_lines_1a_and_8a_are_blank_without_a_1099b` in
    /// `btctax-forms`, which reads the serialized PDF back and asserts the fields are ABSENT.
    #[test]
    fn a_return_with_no_1099b_carries_no_1a_or_8a_amounts() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let st = state_income(vec![mining(dec!(60000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        for (v, what) in [
            (ar.schedule_d.st_1099b_proceeds_1ad, "1a(d)"),
            (ar.schedule_d.st_1099b_cost_1ae, "1a(e)"),
            (ar.schedule_d.lt_1099b_proceeds_8ad, "8a(d)"),
            (ar.schedule_d.lt_1099b_cost_8ae, "8a(e)"),
        ] {
            assert_eq!(v, Usd::ZERO, "line {what} has no 1099-B behind it");
        }
    }

    /// ★★★ §G-28/B4 r1+r2 — THE CRYPTO-DELTA BASELINE MUST SEE THE BROKER POSITION.
    ///
    /// Both review lenses found this independently. `compute_tax_year` prices the crypto slice by
    /// running §1222 twice — with and without the crypto legs — so anything NON-crypto has to be in
    /// the profile or the slice is stacked from the wrong bottom. On $2,000,000 of broker long-term
    /// gain the engine reported ≈$7,946 of crypto-attributable tax against a true ≈$23,800.
    ///
    /// ★★ EXACT EQUALITIES, not bands. The first draft asserted `magi >= 2_105_000` and
    /// `0 < ord_ti < 2_000_000`, which r3 showed were blind to a DOUBLE-COUNT and to moving the
    /// short-term half between the two slices. `cap_gain_distr` is deliberately non-zero here so
    /// `cap_gain_distr + b1099_lt` is distinguishable from `b1099_lt` alone.
    #[test]
    fn the_delta_baseline_sees_non_crypto_capital_gain_and_receipts() {
        let p = ty2024_params();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            div_1099: vec![crate::tax::return_inputs::Form1099Div {
                payer: "Broker LLC".into(),
                box1a_ordinary: dec!(1000),
                box2a_capgain_distr: dec!(5000),
                ..Default::default()
            }],
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                short_term_proceeds: dec!(30000),
                short_term_basis: dec!(10000), // +20,000 short-term
                long_term_proceeds: dec!(2500000),
                long_term_basis: dec!(500000), // +2,000,000 long-term
                basis_reported_and_no_adjustments: Some(true),
            }],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Consulting".into(),
                other_gross_receipts: dec!(85000), // §G-28/B3
                is_sstb: Some(false),
                is_cooperative_patron: Some(false),
                qbi_w2_wages: Some(Usd::ZERO),
                qbi_ubia: Some(Usd::ZERO),
                ..Default::default()
            }),
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let profile = derive_tax_profile(&ri, &p, 2024);

        // box-2a distributions PLUS the broker long-term total — both non-crypto LT-character gain.
        assert_eq!(
            profile.other_net_capital_gain,
            dec!(2005000),
            "5,000 of box-2a distributions + 2,000,000 of broker long-term gain"
        );
        // AGI = ordinary dividends 1,000 + the §1211-limited capital result 2,005,000 + consulting
        // 85,000. ★ The $20,000 SHORT-term broker gain is deliberately ABSENT: `net_1222` has no
        // short-term "other" slot, so the engine's own `without` cannot see it, and putting it here
        // alone would make the profile and the engine disagree (§G-30).
        assert_eq!(profile.magi_excluding_crypto, dec!(2091000));
        // …and the ordinary slice is AGI − standard deduction − QD − the capital result, so the
        // engine can add the capital result back through `without` without double-counting.
        assert_eq!(profile.ordinary_taxable_income, dec!(71400));
    }

    /// ★★★ r4-I3 — THE PROFILE PASSES **ALL SIX** `net_1222` ARGUMENTS THE ENGINE WILL USE.
    ///
    /// The whole fix rests on one claim: *"the profile calls `net_1222` with the SAME arguments the
    /// engine will use, so the two agree by construction rather than by coincidence."* r4 mutated the
    /// call — dropping both carryforwards and hardcoding the §1211(b) limit at $3,000 — and the ENTIRE
    /// workspace stayed green. Four of six arguments were pinned; the two that vary by filer were not.
    ///
    /// ★★ AND THE CARRYFORWARD ARGUMENT WAS NOT COSMETIC. Before this fix `income_total` added a bare
    /// `cap_gain_distr`, so a filer with a capital-loss carryover had a `magi_excluding_crypto` up to
    /// the §1211(b) limit too high — a LIVE wrong AGI on the delta path, which `assemble_absolute` had
    /// right all along. The fix corrected it silently and pinned nothing; this is that pin.
    #[test]
    fn the_profile_passes_every_net_1222_argument_the_engine_uses() {
        let p = ty2024_params();
        let base = |status: FilingStatus, cf_short: Usd, cf_long: Usd, distr: Usd| {
            let mut ri = ReturnInputs {
                filing_status: status,
                header: crate::tax::testonly::kitchen_sink_household().0.header,
                w2s: vec![crate::tax::return_inputs::W2 {
                    owner: Owner::Taxpayer,
                    employer: "ACME".into(),
                    box1_wages: dec!(120000),
                    ..Default::default()
                }],
                div_1099: vec![crate::tax::return_inputs::Form1099Div {
                    payer: "Broker LLC".into(),
                    box2a_capgain_distr: distr,
                    ..Default::default()
                }],
                capital_loss_carryforward_in: crate::tax::types::Carryforward {
                    short: cf_short,
                    long: cf_long,
                },
                ..Default::default()
            };
            crate::tax::testonly::answer_all_live_declarations(&mut ri);
            derive_tax_profile(&ri, &p, 2024)
        };

        // ★ THE CARRYFORWARD PAIR. A $50,000 short-term carryover with no gains to absorb it is a
        //   §1211(b)-limited $3,000 deduction against AGI. Dropping the argument leaves AGI at
        //   $120,000 — $3,000 too high for every carryforward filer.
        assert_eq!(
            base(FilingStatus::Single, dec!(50000), Usd::ZERO, Usd::ZERO).magi_excluding_crypto,
            dec!(117000),
            "a short-term carryover must reduce AGI by the §1211(b)-limited $3,000"
        );
        assert_eq!(
            base(FilingStatus::Single, Usd::ZERO, dec!(50000), Usd::ZERO).magi_excluding_crypto,
            dec!(117000),
            "…and so must a LONG-term one — the second argument is separate from the first"
        );

        // ★★ THE STATUS-DEPENDENT LIMIT. §1211(b)(1) is $1,500 for MFS, not $3,000. Hardcoding the
        //    figure puts an MFS filer's AGI $1,500 too low, and the MFS §1411 threshold is $125,000 —
        //    so the NIIT test flips inside a $1,500 band.
        assert_eq!(
            base(FilingStatus::Mfs, dec!(50000), Usd::ZERO, Usd::ZERO).magi_excluding_crypto,
            dec!(118500),
            "MFS's §1211(b) allowance is HALF — $1,500, not $3,000"
        );

        // ★ …and a carryover is absorbed by distributions before the limit bites, which is what makes
        //   this `net_1222` rather than a subtraction.
        assert_eq!(
            base(FilingStatus::Single, Usd::ZERO, dec!(50000), dec!(20000)).magi_excluding_crypto,
            dec!(117000),
            "$20,000 of distributions absorb $20,000 of the carryover; the residue is still limited"
        );
    }

    /// ★★★ r3-C1 — A BROKER **LOSS** IS §1211(b)-LIMITED BEFORE IT REACHES AGI.
    ///
    /// The fold added the raw signed 1099-B gains to `income_total`. `cap_gain_distr` (1099-DIV box
    /// 2a) is a DISTRIBUTION and can never be negative, so for its whole life that term could be
    /// added unlimited and be right — a broker total can be a LOSS, and adding one raw deducts it
    /// from AGI at full size. §1211(b) caps the deduction at $3,000 ($1,500 MFS).
    #[test]
    fn a_broker_loss_is_limited_to_3000_before_it_reaches_the_delta_baseline() {
        let p = ty2024_params();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            w2s: vec![crate::tax::return_inputs::W2 {
                owner: Owner::Taxpayer,
                employer: "ACME".into(),
                box1_wages: dec!(200000),
                ..Default::default()
            }],
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                long_term_proceeds: Usd::ZERO,
                long_term_basis: dec!(100000), // a $100,000 long-term LOSS
                basis_reported_and_no_adjustments: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let profile = derive_tax_profile(&ri, &p, 2024);
        assert_eq!(
            profile.magi_excluding_crypto,
            dec!(197000),
            "$200,000 of wages less the §1211(b)-limited $3,000 — NOT the whole $100,000 loss"
        );
        assert_eq!(
            profile.other_net_capital_gain,
            dec!(-100000),
            "the raw non-crypto LT position still reaches the engine, which applies §1211 itself"
        );
    }

    /// ★★★ r3-C2 — A LOSS THAT DRIVES AGI BELOW THE DEDUCTION MUST NOT MANUFACTURE ORDINARY INCOME.
    ///
    /// `taxable_income` was `(agi − deduction).max(0)` — already floored — and the fold subtracted the
    /// capital term from THAT. With a negative capital term the subtraction is an ADDITION, so the
    /// clamp discarded the negative and the add-back invented income: W-2 $50,000 with a $200,000
    /// broker long-term loss produced an ordinary base of $200,000 against a true $32,400, pricing
    /// every crypto ordinary dollar at 32-35% instead of 12%.
    #[test]
    fn a_loss_below_the_deduction_does_not_manufacture_ordinary_income() {
        let p = ty2024_params();
        let mk2 = |long: bool, wages: Usd| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                header: crate::tax::testonly::kitchen_sink_household().0.header,
                w2s: vec![crate::tax::return_inputs::W2 {
                    owner: Owner::Taxpayer,
                    employer: "ACME".into(),
                    box1_wages: wages,
                    ..Default::default()
                }],
                b_1099: vec![crate::tax::return_inputs::Form1099B {
                    payer: "Broker LLC".into(),
                    long_term_basis: if long { dec!(200000) } else { Usd::ZERO },
                    short_term_basis: if long { Usd::ZERO } else { dec!(200000) },
                    basis_reported_and_no_adjustments: Some(true),
                    ..Default::default()
                }],
                ..Default::default()
            };
            crate::tax::testonly::answer_all_live_declarations(&mut ri);
            derive_tax_profile(&ri, &p, 2024)
        };
        let mk = |long: bool| mk2(long, dec!(50000));
        // LONG-term loss: AGI = 50,000 − 3,000 = 47,000; ordinary base = 47,000 − 14,600 + 3,000.
        // ★ The `+ 3,000` is the capital term coming back OUT of the ordinary slice, which is right:
        //   the engine subtracts `without.loss_deduction` itself, landing on the true 32,400.
        let lt = mk(true);
        assert_eq!(lt.magi_excluding_crypto, dec!(47000));
        assert_eq!(
            lt.ordinary_taxable_income,
            dec!(35400),
            "50,000 of wages less the 14,600 standard deduction — NOT 200,000"
        );
        // SHORT-term loss: absent from the profile entirely (§G-30), so nothing moves at all.
        let st = mk(false);
        assert_eq!(st.magi_excluding_crypto, dec!(50000));
        assert_eq!(st.ordinary_taxable_income, dec!(35400));

        // ★★★ AND THE CASE THAT ACTUALLY REACHES THE CLAMP. With §1211(b) now limiting the capital
        //     term to −$3,000, AGI can only dip that far below the standard deduction — so the
        //     clamp-before-subtraction bug is reachable ONLY at low income, and the $50,000 fixture
        //     above cannot see it (47,000 clears 14,600 either way). r3 named exactly this class of
        //     vacuous fixture; mutation confirmed my first draft of this test could not red on the
        //     defect in its own name.
        //
        //     Wages $5,000 ⇒ AGI = 5,000 − 3,000 = 2,000, under the $14,600 deduction. Correct:
        //     (2,000 − 14,600 − 0 + 3,000) = −9,600 ⇒ clamped to 0. Clamping FIRST gives
        //     (2,000 − 14,600).max(0) = 0, then +3,000 = $3,000 of income the filer never had.
        let poor = mk2(true, dec!(5000));
        assert_eq!(poor.magi_excluding_crypto, dec!(2000));
        assert_eq!(
            poor.ordinary_taxable_income,
            Usd::ZERO,
            "a filer whose income is below the standard deduction has NO ordinary base — the capital \
             add-back must not manufacture one"
        );
    }

    /// ★★★ §G-28/B4 — 1099-B TOTALS REACH SCHEDULE D LINES 1a/8a AND THE §1222 NETTING.
    ///
    /// The blocker: no 1099-B input existed, so $2,000,000 of stock gain was inexpressible. Schedule D
    /// lines 1a and 8a are the FORM'S OWN totals-without-Form-8949 mechanism, which is why btctax needs
    /// no second lot-level engine for securities.
    #[test]
    fn form_1099b_totals_reach_schedule_d_and_the_1222_netting() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                short_term_proceeds: dec!(150000),
                short_term_basis: dec!(120000),
                long_term_proceeds: dec!(2500000),
                long_term_basis: dec!(500000),
                basis_reported_and_no_adjustments: Some(true),
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let st = crate::state::LedgerState::default();
        assert_eq!(
            crate::tax::return_refuse::screen_inputs(&ri, &table, &p).map(|r| r.reason),
            None,
            "a confirmed 1099-B files"
        );
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        let sd = &ar.schedule_d;
        assert_eq!(sd.st_1099b_proceeds_1ad, dec!(150000), "line 1a(d)");
        assert_eq!(sd.st_1099b_cost_1ae, dec!(120000), "line 1a(e)");
        assert_eq!(sd.st_1099b_gain_1ah, dec!(30000), "line 1a(h) = (d) − (e)");
        assert_eq!(sd.lt_1099b_proceeds_8ad, dec!(2500000), "line 8a(d)");
        assert_eq!(
            sd.lt_1099b_gain_8ah,
            dec!(2000000),
            "line 8a(h) — the $2M the trial could not express"
        );

        // ★★ …and they reach the NETTING, so 1040 line 7 and the preferential-rate stack see them.
        assert_eq!(sd.st_net_7, dec!(30000), "line 7 combines 1a through 6");
        assert_eq!(
            sd.lt_net_15,
            dec!(2000000),
            "line 15 combines 8a through 14"
        );
        assert_eq!(ar.capital_gain, dec!(2030000), "1040 line 7");
        assert!(
            ar.printed_inputs.qbi_net_capital_gain >= dec!(2000000),
            "the §199A net-capital-gain limitation must see the long-term gain too"
        );

        // ★★★ AND THE PRINTED SCHEDULE D, which is a DIFFERENT chain from the core nets above.
        //
        //     `st_net_7` is `CapNet::st_net`; `ScheduleDLines::line7` is *"Combine lines 1a through 6
        //     in column (h)"* over the PRINTED cells. Asserting only the first left both printed
        //     combinations unheld — deleting `line1a_h` from line 7 and `line8a_h` from line 15 red
        //     NOTHING until this block existed. Same field-of-view failure as the 1040-vs-8995-A pair.
        let pr = crate::tax::packet::assemble_printed_return(
            &ri,
            &st,
            &std::collections::BTreeMap::new(),
            &ar,
            &table,
            2024,
            &[],
        )
        .expect("the printed return assembles");
        let d = &pr.forms.sch_d;
        assert_eq!(d.line1a_d, dec!(150000), "printed line 1a(d)");
        assert_eq!(d.line1a_e, dec!(120000), "printed line 1a(e)");
        assert_eq!(d.line1a_h, dec!(30000), "printed line 1a(h)");
        assert_eq!(d.line8a_d, dec!(2500000), "printed line 8a(d)");
        assert_eq!(d.line8a_h, dec!(2000000), "printed line 8a(h)");
        assert_eq!(
            d.line7, d.line1a_h,
            "line 7 COMBINES line 1a — with no 8949 rows it IS line 1a(h)"
        );
        assert_eq!(d.line15, d.line8a_h, "line 15 COMBINES line 8a — likewise");
        assert_eq!(
            d.line16,
            d.line7 + d.line15,
            "line 16 = 7 + 15, over the printed cells"
        );
        // …and the 1040 takes the same figure.
        assert_eq!(
            pr.forms.f1040.line7, ar.capital_gain,
            "1040 line 7 = Schedule D's answer"
        );
    }

    /// ★★★ THE FORM'S OWN TWO CONDITIONS, AND THEY FAIL CLOSED.
    ///
    /// Schedule D line 1a is available only *"for which basis was reported to the IRS and for which
    /// you have no adjustments"*. Anything else needs Form 8949 with Box B/C/E/F and per-transaction
    /// detail — the lot-level engine btctax will not build for securities. `None` (never asked) and
    /// `Some(false)` must BOTH refuse, or an omission becomes a claim.
    #[test]
    fn a_1099b_that_does_not_qualify_for_line_1a_refuses() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mk = |gate: Option<bool>, amount: Usd| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                header: crate::tax::testonly::kitchen_sink_household().0.header,
                b_1099: vec![crate::tax::return_inputs::Form1099B {
                    payer: "Broker LLC".into(),
                    long_term_proceeds: amount,
                    long_term_basis: Usd::ZERO,
                    basis_reported_and_no_adjustments: gate,
                    ..Default::default()
                }],
                ..Default::default()
            };
            crate::tax::testonly::answer_all_live_declarations(&mut ri);
            ri
        };
        for gate in [None, Some(false)] {
            let r = crate::tax::return_refuse::screen_inputs(&mk(gate, dec!(2500000)), &table, &p)
                .unwrap_or_else(|| panic!("{gate:?} must REFUSE — an omission is not a claim"));
            assert_eq!(r.reason, RefuseReason::Form1099BNeedsForm8949);
            assert!(
                r.detail.contains("Broker LLC") && r.detail.contains("Form 8949"),
                "the refusal must name the broker and where those transactions belong: {}",
                r.detail
            );
        }
        // ★ …and it is NOT always-on: an affirmative `yes` files.
        assert_eq!(
            crate::tax::return_refuse::screen_inputs(&mk(Some(true), dec!(2500000)), &table, &p)
                .map(|r| r.reason),
            None
        );
        // ★★ …nor does an ALL-ZERO row refuse. It asserts nothing and reports nothing, so demanding a
        //    confirmation for it would be a refusal with no purpose.
        assert_eq!(
            crate::tax::return_refuse::screen_inputs(&mk(None, Usd::ZERO), &table, &p)
                .map(|r| r.reason),
            None,
            "an empty 1099-B row carries no testimony and must not gate the return"
        );
    }

    /// ★★★ §G-28/B4 r1-Minor — THE BROKER TOTALS NET AGAINST A NONZERO **CRYPTO** POSITION.
    ///
    /// Every other B4 test runs on an EMPTY ledger, so the claim that the totals "join the crypto
    /// nets" was never observed with a crypto net to join: mutating `capital_net` to drop the crypto
    /// addend entirely — `net_1222(b_st, b_lt, …)` — survived all of them.
    ///
    /// This is the arrangement B4's own sentence describes: a broker loss against a crypto gain of the
    /// SAME character, netting within character before the §1211(b) limit, which is Schedule D's order
    /// (lines 1a‥6 → 7; 8a‥14 → 15; then 16).
    #[test]
    fn broker_totals_net_against_a_nonzero_crypto_position() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                // A $40,000 LONG-TERM broker LOSS, against the ledger's long-term crypto GAIN.
                long_term_proceeds: dec!(10000),
                long_term_basis: dec!(50000),
                basis_reported_and_no_adjustments: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        // A $100,000 LONG-TERM crypto gain from the ledger.
        let st = state_disposals(vec![disp_leg(Term::LongTerm, dec!(150000), dec!(50000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);

        // The fixture must have a REAL crypto net, or the test is the empty-ledger one again.
        let crypto_only = assemble_absolute(
            &{
                let mut r = ri.clone();
                r.b_1099.clear();
                r
            },
            &st,
            &p,
            &table,
            2024,
        );
        assert_eq!(
            crypto_only.capital_gain,
            dec!(100000),
            "the ledger alone must produce a $100,000 long-term gain, else this is vacuous"
        );
        // …and the broker loss nets against it WITHIN character, before anything else.
        assert_eq!(
            ar.schedule_d.lt_1099b_gain_8ah,
            dec!(-40000),
            "line 8a(h) is the broker's own signed total"
        );
        assert_eq!(
            ar.capital_gain,
            dec!(60000),
            "$100,000 crypto LT gain − $40,000 broker LT loss = $60,000, netted within character"
        );
    }

    /// ★★ A 1099-B LOSS nets within its own character and is limited by §1211(b) like any other.
    #[test]
    fn a_1099b_loss_nets_within_character_and_hits_the_1211_limit() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker LLC".into(),
                short_term_proceeds: dec!(10000),
                short_term_basis: dec!(60000), // a $50,000 short-term LOSS
                basis_reported_and_no_adjustments: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let ar = assemble_absolute(&ri, &crate::state::LedgerState::default(), &p, &table, 2024);
        assert_eq!(
            ar.schedule_d.st_1099b_gain_1ah,
            dec!(-50000),
            "line 1a(h) is SIGNED — a broker's netted totals are a loss as readily as a gain"
        );
        assert_eq!(
            ar.capital_gain,
            dec!(-3000),
            "§1211(b) limits the deductible loss to $3,000 for a single filer"
        );
    }

    /// ★★★ §G-28/B3 — NON-LEDGER GROSS RECEIPTS REACH SCHEDULE C LINE 1, SCHEDULE SE AND §199A.
    ///
    /// The blocker: `ScheduleCInputs` had no gross-receipts field, so Schedule C revenue could only
    /// arrive as mined Bitcoin and a filer with $85,000 of consulting income could not represent it
    /// at all. Line 1 is now the SUM, and this pins that the sum reaches all three consumers.
    ///
    /// ★★ THE EQUIVALENCE THIS EXISTS TO PIN. `compute_se_tax` is in the FROZEN delta engine, so the
    /// receipts reach it through its ONE arithmetic use of the expenses argument
    /// (`n = gross_se − expenses`), passed as `expenses − receipts`. This asserts the resulting net SE
    /// equals the direct figure — Schedule C line 1 minus line 28 — rather than trusting the algebra.
    #[test]
    fn non_ledger_receipts_reach_schedule_se_exactly() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mk = |mined: Usd, other: Usd, expenses: Usd| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                header: crate::tax::testonly::kitchen_sink_household().0.header,
                schedule_c: Some(ScheduleCInputs {
                    owner: Owner::Taxpayer,
                    business_description: "Consulting and mining".into(),
                    expenses,
                    other_gross_receipts: other,
                    is_sstb: Some(false),
                    is_cooperative_patron: Some(false),
                    qbi_w2_wages: Some(Usd::ZERO),
                    qbi_ubia: Some(Usd::ZERO),
                    ..Default::default()
                }),
                ..Default::default()
            };
            crate::tax::testonly::answer_all_live_declarations(&mut ri);
            let st = if mined > Usd::ZERO {
                state_income(vec![mining(mined)])
            } else {
                crate::state::LedgerState::default()
            };
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            (ri, st, ar)
        };

        // (a) $40,000 mined + $85,000 consulting − $5,000 expenses. Line 1 = $125,000.
        let (_, _, ar) = mk(dec!(40000), dec!(85000), dec!(5000));
        let sc = ar.schedule_c.as_ref().expect("a Schedule C files");
        assert_eq!(
            sc.gross_receipts_1,
            dec!(125000),
            "line 1 is the LEDGER plus the filer's non-ledger receipts"
        );
        assert_eq!(sc.net_profit_31, dec!(120000), "line 31 = line 1 − line 28");
        let se = ar.se.as_ref().expect("SE tax applies");
        assert_eq!(
            se.net_se, dec!(120000),
            "★ Schedule SE's net earnings are line 1 − line 28, NOT the ledger's gross − expenses. \
             This is the whole equivalence: it is reached through the frozen function's expenses \
             argument, and it must land on the same number."
        );

        // (b) ★★★ A PURELY NON-CRYPTO BUSINESS — the case the blocker made inexpressible. No ledger
        //     income at all, so `se_net_income` returns zero and every figure comes from the filer.
        let (_, _, ar) = mk(Usd::ZERO, dec!(85000), dec!(5000));
        let sc = ar
            .schedule_c
            .as_ref()
            .expect("a Schedule C files with no crypto at all");
        assert_eq!(sc.gross_receipts_1, dec!(85000));
        assert_eq!(sc.net_profit_31, dec!(80000));
        assert_eq!(
            ar.se
                .as_ref()
                .expect("SE tax applies to non-crypto self-employment too")
                .net_se,
            dec!(80000)
        );
        // …and it earns the §199A deduction like any other qualified trade or business.
        assert!(
            ar.printed_inputs.business_qbi > Usd::ZERO,
            "non-crypto self-employment is a qualified trade or business too"
        );

        // (c) The regression guard: with NO non-ledger receipts nothing moves.
        let (_, _, ar) = mk(dec!(40000), Usd::ZERO, dec!(5000));
        let sc = ar.schedule_c.as_ref().unwrap();
        assert_eq!(sc.gross_receipts_1, dec!(40000));
        assert_eq!(sc.net_profit_31, dec!(35000));
        assert_eq!(ar.se.as_ref().unwrap().net_se, dec!(35000));
    }

    /// ★★ Non-ledger receipts can COVER the expenses that would otherwise be a Schedule C loss.
    ///
    /// The loss screen used to test `ledger gross − expenses`, so a filer whose consulting revenue
    /// paid for their mining costs was refused for a loss they did not have.
    #[test]
    fn consulting_revenue_covers_mining_expenses_instead_of_refusing_a_loss() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Consulting and mining".into(),
                expenses: dec!(30000), // more than the mined income…
                other_gross_receipts: dec!(50000), // …but the consulting revenue covers it
                is_sstb: Some(false),
                is_cooperative_patron: Some(false),
                qbi_w2_wages: Some(Usd::ZERO),
                qbi_ubia: Some(Usd::ZERO),
                ..Default::default()
            }),
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let st = state_income(vec![mining(dec!(20000))]);
        // ★★★ `screen_compute_dependent` IS THE SCREEN THAT HOLDS THE LOSS TEST, and calling
        //     `screen_absolute` instead is why the first draft of this test let the mutation live:
        //     reverting the loss check to `ledger gross − expenses` did not red anything. A test that
        //     names the wrong screen is an untested guard wearing a green tick.
        assert_eq!(
            screen_compute_dependent(&ri, &st, 2024, &p).map(|r| r.reason),
            None,
            "$20k mined + $50k consulting − $30k expenses is a $40,000 PROFIT, not a loss"
        );
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "…and nothing downstream refuses it either"
        );
        assert_eq!(ar.schedule_c.as_ref().unwrap().net_profit_31, dec!(40000));

        // ★★ And the guard is NOT always-off: the same business WITHOUT the consulting revenue is a
        //    genuine loss and must still refuse.
        let mut loss = ri.clone();
        loss.schedule_c.as_mut().unwrap().other_gross_receipts = Usd::ZERO;
        assert_eq!(
            screen_compute_dependent(&loss, &st, 2024, &p).map(|r| r.reason),
            Some(RefuseReason::ScheduleCLoss),
            "$20k mined − $30k expenses with no other revenue IS a loss, and still refuses"
        );
    }

    /// ★★★ §G-28/B1b — THIS RETURN NO LONGER REFUSES. It is the case the blocker existed for.
    ///
    /// A sole proprietor above the §199A(e)(2) threshold with no employees and no qualified property:
    /// §199A(b)(2) caps the deduction at the greater of 50% of W-2 wages ($0) and 25% of wages plus
    /// 2.5% of UBIA ($0), so the cap is **zero** and so is the deduction. That is a real answer,
    /// figured from two numbers the filer can state — and the old blanket `QbiAboveThreshold` refusal
    /// was refusing a return Form 8995-A tells us exactly how to complete.
    ///
    /// ★★ The assertion is deliberately BOTH halves: the return computes, AND the deduction is the
    /// figure the cap produces. A test that only asserted "no refusal" would pass just as well if the
    /// deduction silently became the uncapped 20% of QBI — which is the overstating direction, and
    /// the direction the refusal was protecting against in the first place.
    #[test]
    fn an_over_threshold_schedule_c_with_no_wages_now_files_with_a_zero_deduction() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Bitcoin mining".into(),
                is_sstb: Some(false),
                is_cooperative_patron: Some(false),
                // ★ ANSWERED as zero, which is a real fact about a solo miner — not a default. An
                //   UNANSWERED pair still refuses; `an_unanswered_wage_or_ubia_figure_refuses` covers it.
                qbi_w2_wages: Some(Usd::ZERO),
                qbi_ubia: Some(Usd::ZERO),
                ..Default::default()
            }),
            ..Default::default()
        };
        // $400,000 of mined crypto ⇒ TI-before-QBI ≈ $369,590, clear of the $241,950 TOP of the single
        // phase-in range, and there is not a single REIT dividend on the return.
        //
        // ★ $260,000 was the figure the old refusal test used, and it lands INSIDE the range (≈
        //   $231,465) — where Part III correctly yields a PARTIAL deduction rather than zero. Worth
        //   recording: the first draft of this test asserted the above-the-range answer on an
        //   in-range fixture and read as a bug in the code.
        let st = state_income(vec![mining(dec!(400000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);

        assert!(
            ar.printed_inputs.business_qbi > Usd::ZERO,
            "the fixture must actually produce business QBI, else this test is vacuous"
        );
        assert!(
            ar.agi - ar.deduction > p.qbi_phase_in_top(FilingStatus::Single),
            "TI-before-QBI must clear the TOP of the phase-in range, else Part III softens the cap \
             and this test is asserting the wrong regime's answer"
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "an over-threshold Schedule C with no wages and no UBIA is COMPUTABLE — Form 8995-A \
             Parts I-III figure the §199A(b)(2) cap from two numbers the filer stated"
        );
        // ★★ …and the cap is what produced the answer, not an uncapped 20% of QBI.
        let f = ar
            .f8995a_parts_i_to_iii
            .as_ref()
            .expect("a trade or business above the threshold files Parts I-III");
        assert!(
            f.part_ii.line3 > Usd::ZERO,
            "line 3 (20% of QBI) must be non-zero, else the cap has nothing to bind against"
        );
        assert_eq!(
            f.part_ii.line10,
            Usd::ZERO,
            "line 10 — the greater of 50%×0 wages and 25%×0 + 2.5%×0 UBIA"
        );
        assert_eq!(
            f.part_ii.line16,
            Usd::ZERO,
            "so the qualified business income component is ZERO, not 20% of QBI"
        );
        assert!(
            ar.uses_8995a,
            "and it files Form 8995-A, not the simplified 8995"
        );
    }

    /// ★★★ §G-28/B1b — THE 1040 AND THE ATTACHED FORM MUST CLAIM THE SAME DEDUCTION.
    ///
    /// Two Criticals lived in this one sentence, and BOTH were found by reading the emitted packet
    /// rather than by any test that passed:
    ///
    ///   1. **The 1040 took no deduction at all.** Line 13 read only Form 8995's line 15, and on the
    ///      8995-A path `f8995` is `None` (the two are alternatives). So the 1040 printed ZERO while
    ///      the stapled Form 8995-A line 39 claimed $27,357 — the return disagreed with its own
    ///      attachment, and OVERSTATED the filer's tax by the whole deduction. Shipped by B1a.
    ///   2. **The deduction was UNCAPPED.** `compute_8995` recomputed a flat 20% of QBI, so Part IV
    ///      line 27 ("Enter the amount from line 16") printed $45,267 against its own line 16 of
    ///      $27,357 — a $17,910 overstated deduction, in the UNDERSTATING direction for tax.
    ///
    /// ★★ This asserts the IDENTITY, not either figure. A test that pinned only the number would have
    /// passed on defect 1 the moment someone "fixed" line 39 to match the zero.
    #[test]
    fn the_1040_deduction_equals_the_attached_8995a_line_39() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        // A single filer INSIDE the phase-in range with no employees: the cap binds, Part III softens
        // it, and the answer is neither 20% of QBI nor zero — so all three candidates are distinct.
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            // A real header — the printed packet will not assemble a return that names nobody.
            header: crate::tax::testonly::kitchen_sink_household().0.header,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Bitcoin mining".into(),
                is_sstb: Some(false),
                is_cooperative_patron: Some(false),
                qbi_w2_wages: Some(Usd::ZERO),
                qbi_ubia: Some(Usd::ZERO),
                ..Default::default()
            }),
            ..Default::default()
        };
        // The printed packet REFUSES on any unanswered declaration, so this fixture answers them all
        // — the subject here is the §199A identity, not the answered-ness screen.
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let st = state_income(vec![mining(dec!(240000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "this return must FILE, else the test proves nothing"
        );
        let pr = crate::tax::packet::assemble_printed_return(
            &ri,
            &st,
            &std::collections::BTreeMap::new(),
            &ar,
            &table,
            2024,
            &[],
        )
        .expect("the printed return assembles");
        let a = pr
            .forms
            .f8995a
            .as_ref()
            .expect("above the threshold ⇒ Form 8995-A");
        let parts = a
            .parts_i_to_iii
            .as_ref()
            .expect("a trade or business ⇒ Parts I-III");

        // ★ The fixture must actually EXERCISE the cap and the phase-in, or the three candidate
        //   figures collapse into one and the identity below is trivially true.
        assert!(parts.part_iii.is_some(), "the fixture must run Part III");
        assert!(
            parts.part_ii.line16 < parts.part_ii.line3,
            "the cap must BIND (line 16 < 20% of QBI), else this fixture cannot tell the two \
             defects apart"
        );
        assert!(
            parts.part_ii.line16 > Usd::ZERO,
            "…and must not be zero either"
        );

        // 1. Part IV line 27 IS line 16 — the form's own words.
        assert_eq!(
            a.part_iv.line27, parts.part_ii.line16,
            "Part IV line 27 is \"Enter the amount from line 16\""
        );
        // 2. The 1040 takes what the attached form claims.
        assert_eq!(
            pr.forms.f1040.line13, a.part_iv.line39,
            "1040 line 13 is \"Qualified business income deduction from Form 8995 OR Form 8995-A\""
        );
        // 3. …and the engine's own deduction is the same figure, so the tax computed matches the
        //    tax the printed page justifies.
        assert_eq!(
            ar.qbi_deduction, a.part_iv.line39,
            "the engine's deduction and the printed one are the same number"
        );
        assert!(
            pr.forms.f8995.is_none(),
            "and the simplified form is NOT also filed"
        );
    }

    /// ★★★ An UNANSWERED W-2-wage or UBIA figure still refuses above the threshold.
    ///
    /// `assemble_absolute` defaults both to zero when building Parts I–III, and that default is safe
    /// ONLY because this refusal stands in front of it. A defaulted zero would CAP the deduction at
    /// zero for a filer who does pay wages — overstating their tax — and any other default would
    /// invent wages they never reported. Neither direction is safe, so `None` refuses at the point of
    /// need. This test is what reds if the refusal is ever narrowed away and the default is left.
    #[test]
    fn an_unanswered_wage_or_ubia_figure_refuses_above_the_threshold() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let st = state_income(vec![mining(dec!(260000))]);
        for (wages, ubia, which) in [
            (None, Some(Usd::ZERO), "W-2 wages"),
            (Some(Usd::ZERO), None, "UBIA"),
            (None, None, "both"),
        ] {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                schedule_c: Some(ScheduleCInputs {
                    owner: Owner::Taxpayer,
                    business_description: "Bitcoin mining".into(),
                    is_sstb: Some(false),
                    is_cooperative_patron: Some(false),
                    qbi_w2_wages: wages,
                    qbi_ubia: ubia,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            let r = screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).expect("must refuse");
            assert_eq!(
                r.reason,
                RefuseReason::QbiAboveThreshold,
                "an unanswered {which} above the threshold must REFUSE, not default to zero"
            );
            // ★★★ AND THE WAY OUT MUST BE A COMMAND THAT CAN ACTUALLY COLLECT THE ANSWER.
            //
            //     `btctax income answer` walks the DECLARATION and SKIPPABLE registries — bools and
            //     dates. It will never ask for a money amount, so naming it here refuses the filer
            //     and then sends them somewhere that cannot help: they run it, it completes, and the
            //     return still refuses. Same trap the Schedule B line 7b country-names refusal names.
            assert!(
                !r.detail.contains("income answer"),
                "{which}: the exit must not be `income answer`, which collects no money amounts: {}",
                r.detail
            );
            assert!(
                r.detail.contains("income import")
                    && r.detail.contains("qbi_w2_wages")
                    && r.detail.contains("qbi_ubia"),
                "{which}: the exit must NAME the two fields and the command that takes them: {}",
                r.detail
            );
        }
    }

    /// ★★★ §G-28/B1b — ABOVE the §199A(e)(2) threshold an UNANSWERED SSTB checkbox refuses, and it
    /// refuses BEFORE the wage/UBIA one.
    ///
    /// Past the phase-in range an SSTB's QBI is excluded **entirely** (§199A(d)(3)), so an unasked
    /// `no` hands the filer a deduction the statute denies. It is asked first because if the answer
    /// is `yes` the deduction is zero regardless of wages, and demanding two figures that cannot
    /// matter is the wrong question.
    #[test]
    fn an_unanswered_sstb_refuses_before_the_wage_ubia_refusal() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Bitcoin mining".into(),
                is_cooperative_patron: Some(false), // answered; it precedes the SSTB mandate
                is_sstb: None,                      // ← the only difference from the vector above
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![mining(dec!(260000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        let r = screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).expect("must refuse");
        assert_eq!(
            r.reason,
            RefuseReason::SstbUnanswered,
            "the SSTB question is mandatory above the threshold, and precedes the wage/UBIA refusal"
        );
        assert!(
            r.detail.contains("§199A(d)(3)") && r.detail.contains("income answer"),
            "the refusal must name the statute and the command that answers it: {}",
            r.detail
        );
    }

    /// ★★★ ...and BELOW the threshold the very same blank does NOT refuse.
    ///
    /// This is the `DonationsHadRestrictions` shape — *offered always, mandatory only where it
    /// matters*. Under the threshold §199A is the simplified Form 8995, which has no SSTB checkbox at
    /// all, so the answer changes nothing and demanding it would be a refusal with no purpose. A
    /// draft that made this a live class-(A) declaration refused **every Schedule C return at any
    /// income**; this test is what reds if that regresses.
    #[test]
    fn below_the_threshold_an_unanswered_sstb_does_not_refuse() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Bitcoin mining".into(),
                is_cooperative_patron: Some(false),
                is_sstb: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let st = state_income(vec![mining(dec!(60000))]);
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        assert!(
            ar.printed_inputs.business_qbi > Usd::ZERO,
            "the fixture must produce business QBI, else this test is vacuous"
        );
        assert!(
            ar.agi - ar.deduction < p.qbi_ti_threshold(FilingStatus::Single),
            "TI-before-QBI must be UNDER the threshold, else this test proves the opposite"
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "under the threshold the SSTB answer is irrelevant — Form 8995 has no such checkbox"
        );
    }

    /// **H1 — the household the lift admits.** Single, $5,000 of wages, a $2,000 SHORT-TERM capital
    /// loss carried IN. AGI = 5,000 + (−2,000) = 3,000; the $14,600 standard deduction wipes it out,
    /// so taxable income is $0 and there is a carryforward-in. That is precisely the pair that used to
    /// refuse.
    use crate::tax::printed::ScheduleDRouting;

    fn h1_files_at_the_floor() -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(5000), dec!(5000), dec!(5000))],
            ..Default::default()
        };
        ri.capital_loss_carryforward_in = Carryforward {
            short: dec!(2000),
            long: Usd::ZERO,
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        ri
    }

    /// ★★★ **K1 — (A). A taxable-income-$0 year with a capital-loss carryover brought IN now FILES,
    /// and its Schedule D is indistinguishable from its current-year-loss twin's.**
    ///
    /// This REPLACES `taxable_income_nonpositive_with_carryforward_refuses`, which asserted the
    /// opposite. The refusal was never about arithmetic — `capital_loss_carryover.rs` transcribes all
    /// thirteen worksheet lines and a carryforward-IN has always entered `net_1222` before Schedule D
    /// lines 7/15/16/21 are formed. It was about a DECISION: emitting a 1040 for this household
    /// widens the filing surface on a return signed under §6065. The owner took it.
    ///
    /// ★ **The twin comparison is the point.** A $2,000 loss carried in and a $2,000 loss realised
    /// this year produce the SAME Schedule D Part III routing and the same §1211(b) line 21 — the
    /// form does not distinguish them, and after the lift neither does btctax. A refusal keyed to one
    /// of them was a distinction the form never made.
    ///
    /// Mutations that MUST red:
    ///   (a) restore the deleted `screen_absolute` block ⇒ the files-half reds;
    ///   (b) `st_carryover_6: Usd::ZERO` in `assemble_absolute` ⇒ the L6/L7/L16/L21 assertions red,
    ///       which proves this test READS the carryover rather than merely the routing.
    #[test]
    fn a_ti_zero_carryforward_in_return_files_and_matches_its_current_year_twin() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = h1_files_at_the_floor();

        // ── PREMISE: the fixture really is at the floor WITH a carryover in. ───────────────────────
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            ar.taxable_income,
            Usd::ZERO,
            "premise: the household must sit ON the floor, or the lifted screen is never reached"
        );
        assert!(
            ri.capital_loss_carryforward_in.short > Usd::ZERO,
            "premise: it must bring a loss IN, or this is a plain refund-only filer"
        );

        // ── IT FILES — both screens. ──────────────────────────────────────────────────────────────
        assert_eq!(
            crate::tax::return_refuse::screen_inputs(&ri, &table, &p).map(|r| r.reason),
            None,
            "the worksheet's two header declarations are answered, so nothing at the input screen \
             stands in the way"
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "★ (A): taxable income of $0 with a carryforward brought in FILES"
        );

        // ── Schedule D carries the loss the filer brought in. ──────────────────────────────────────
        let pf = crate::tax::packet::assemble_printed_forms(
            &ri,
            &empty_ledger(),
            &BTreeMap::new(),
            &ar,
            &table,
            2024,
            &[],
        );
        assert_eq!(pf.sch_d.line6, dec!(2000), "L6 — the carryover, paren box");
        assert_eq!(pf.sch_d.line7, dec!(-2000), "L7 — net short-term");
        assert_eq!(pf.sch_d.line16, dec!(-2000), "L16 — 7 and 15 combined");
        assert_eq!(
            pf.sch_d.routing,
            ScheduleDRouting::NetLoss {
                line21: dec!(2000),
                line22_yes: false,
            },
            "L21 — the whole $2,000 is inside the §1211(b) $3,000 allowance"
        );
        assert_eq!(
            ar.capital_loss_carryforward_out,
            Carryforward {
                short: dec!(2000),
                long: Usd::ZERO,
            },
            "★ the worksheet: line 1 = −11,600 ⇒ line 3 = -0- ⇒ line 4 = -0-, so NOTHING was \
             absorbed and the whole $2,000 survives. The flat rule would have said $0."
        );
        assert!(
            pf.sch_d.must_file(),
            "a Schedule D with a carryover must file"
        );

        // ── THE TWIN: the same $2,000 loss REALISED this year, no carryover. ──────────────────────
        let mut twin = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(5000), dec!(5000), dec!(5000))],
            b_1099: vec![crate::tax::return_inputs::Form1099B {
                payer: "Broker".into(),
                short_term_proceeds: dec!(1000),
                short_term_basis: dec!(3000),
                basis_reported_and_no_adjustments: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut twin);
        let ar_twin = assemble_absolute(&twin, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            ar_twin.taxable_income,
            Usd::ZERO,
            "premise: the twin is at the floor too"
        );
        assert_eq!(
            screen_absolute(&twin, &ar_twin, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "premise: the twin has ALWAYS filed — it is the asymmetry that was the finding"
        );
        let pf_twin = crate::tax::packet::assemble_printed_forms(
            &twin,
            &empty_ledger(),
            &BTreeMap::new(),
            &ar_twin,
            &table,
            2024,
            &[],
        );
        assert_eq!(
            pf.sch_d.routing, pf_twin.sch_d.routing,
            "★ the two households route Part III identically — the form draws no line between a \
             loss brought in and a loss realised, and neither does btctax now"
        );
        assert_eq!(pf.f1040.line7, pf_twin.f1040.line7, "1040 L7 agrees too");
        assert_eq!(pf.f1040.line15, pf_twin.f1040.line15, "…and 1040 L15");
    }

    /// ★★★ **K3 — H9: the only newly-admitted household that OWES, and the only vector where
    /// worksheet line 4 is STRICTLY BETWEEN zero and the §1211(b) allowance.**
    ///
    /// Schedule C gross $18,000, a $45,000 long-term capital loss carried in. The §1211(b) allowance
    /// and the §164(f) half-SE deduction land taxable income just below zero, so the year absorbs
    /// SOME of the allowance but not all of it — `line4 = line2.min(line3)` with both operands
    /// distinct. Every other vector in this module sits at one end or the other, where a wrong
    /// `line4` is invisible.
    ///
    /// ★ And it OWES: self-employment tax is not reduced by a capital loss, so a household with $0 of
    /// taxable income still writes a cheque. The lift admits a filer who owes money, not only one who
    /// is owed a refund — which is the shape that makes the widening consequential.
    ///
    /// ★★ The PERSISTED figure is the whole-dollar one. The exact worksheet line 13 is $42,871.66;
    /// what is stored — and what next year's Schedule D lines 6 and 14 will carry — is $42,872.
    ///
    /// Mutations that MUST red, BOTH needed because each pins one side of the partial:
    ///   (a) `line4 = line2` (the flat rule) ⇒ line 13 becomes 42,000;
    ///   (b) `line4 = Usd::ZERO` (carry everything at the floor) ⇒ line 13 becomes 45,000.
    #[test]
    fn the_owing_household_at_the_floor_files_and_owes() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_c: Some(crate::tax::return_inputs::ScheduleCInputs {
                owner: Owner::Taxpayer,
                business_description: "Bitcoin mining".into(),
                naics_code: "518210".into(),
                other_gross_receipts: dec!(18000),
                // Skippables — a trade or business makes both §199A questions live, and they are
                // answered here so this test keeps testing the §1212(b) worksheet.
                is_sstb: Some(false),
                is_cooperative_patron: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        ri.capital_loss_carryforward_in = Carryforward {
            short: Usd::ZERO,
            long: dec!(45000),
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);

        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);

        // ── PREMISES. ─────────────────────────────────────────────────────────────────────────────
        assert_eq!(
            crate::tax::return_refuse::screen_inputs(&ri, &table, &p).map(|r| r.reason),
            None
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "premise: H9 FILES — before the lift this household was refused"
        );
        assert_eq!(ar.taxable_income, Usd::ZERO, "premise: it is AT the floor");

        let w = ar
            .capital_loss_carryover_worksheet
            .expect("the worksheet applies: 1040 line 15 would be below zero");
        // ── THE PARTIAL. ──────────────────────────────────────────────────────────────────────────
        assert!(
            w.line4 > Usd::ZERO && w.line4 < w.line2,
            "★★★ THE POINT OF THIS VECTOR: line 4 is STRICTLY between zero and the §1211(b) \
             allowance — {} against a line 2 of {}. A rule that carried everything at the floor and \
             a rule that always absorbed the full allowance BOTH pass every other vector here.",
            w.line4,
            w.line2
        );
        assert_eq!(w.line1, dec!(-871.66), "1040 line 15, unfloored");
        assert_eq!(w.line3, dec!(2128.34), "line 1 + line 2, floored at zero");
        assert_eq!(w.line4, dec!(2128.34), "the smaller of line 2 and line 3");
        assert_eq!(w.line11, Some(dec!(2128.34)), "line 4 − line 5");
        assert_eq!(w.line13, Some(dec!(42871.66)), "45,000 − 2,128.34");

        // ── IT OWES — self-employment tax, which no capital loss reduces. ─────────────────────────
        assert!(
            ar.total_tax > Usd::ZERO,
            "★ a $0-taxable-income household that still writes a cheque: {}",
            ar.total_tax
        );

        // ── AND THE PERSISTED FIGURE IS THE WHOLE-DOLLAR ONE. ────────────────────────────────────
        let next = apply_carryover_writeback(
            &ar,
            &ri,
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .expect("the write-back succeeds");
        assert_eq!(
            next.capital_loss_carryforward_in,
            Carryforward {
                short: Usd::ZERO,
                long: dec!(42872),
            },
            "★★ $42,871.66 exact becomes $42,872 stored — the figure the filer will read off next \
             year's Schedule D line 14 and sign for"
        );
    }

    /// ★★★ **K2 — (A)'s CENTRAL SAFETY CLAIM: the lift moves no printed line.**
    ///
    /// Deleting a screen changes what is EMITTED, never what is COMPUTED — `screen_absolute` runs
    /// after `assemble_absolute` and reads it. The way to get that wrong is to "lift" the refusal by
    /// relaxing the thing that made taxable income zero, and the standing temptation is the
    /// `max(Usd::ZERO)` floor on 1040 line 15: drop it and the household stops being at the floor, so
    /// the screen stops firing — for entirely the wrong reason, and with a NEGATIVE line 15 on the
    /// filed page.
    ///
    /// So the whole printed 1040 and the whole printed Schedule D are frozen, as STRUCT LITERALS: a
    /// new field on either is then an `E0063` here, which forces a human to state its expected value
    /// rather than letting it default into the pinned set.
    ///
    /// Mutation that MUST red: drop `.max(Usd::ZERO)` from `line15` in `printed::form_1040_lines`
    /// ⇒ the printed page carries −11,600 and this reds, while
    /// `a_ti_zero_carryforward_in_return_files_and_matches_its_current_year_twin` stays GREEN (both
    /// households move together, so its twin comparison cannot see it). This test is the only one
    /// that catches it.
    ///
    /// ★★ **TWO MUTATIONS WERE TRIED AND ONE DID NOT DISCRIMINATE — recorded, because a mutation
    /// that passes is a fact about the code.** Dropping `.max(Usd::ZERO)` from `taxable_income` in
    /// `assemble_absolute` leaves every printed line on this household UNCHANGED: the printed chain
    /// re-derives line 15 from printed lines 11 and 14 **with its own floor**
    /// (`printed.rs`), and the tax on a negative taxable income is zero either way. So the filed page
    /// is floored twice, independently — good news, but it means the absolute-side floor is NOT what
    /// this test guards, and claiming it was would have credited this KAT with a kill it does not
    /// make.
    ///
    /// ★ The plan's other suggested mutation — flooring `form_1040_line15_signed` to `taxable_income`
    /// — also does not belong here: it moves the worksheet's carryover-OUT, which is not a printed
    /// line on any form. K1's `capital_loss_carryforward_out` assertion is what catches that one.
    #[test]
    fn the_lift_moves_no_printed_line() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = h1_files_at_the_floor();
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        let pf = crate::tax::packet::assemble_printed_forms(
            &ri,
            &empty_ledger(),
            &BTreeMap::new(),
            &ar,
            &table,
            2024,
            &[],
        );

        let z = Usd::ZERO;
        assert_eq!(
            pf.sch_d,
            crate::tax::printed::ScheduleDLines {
                line1a_d: z,
                line1a_e: z,
                line1a_h: z,
                line3_d: z,
                line3_e: z,
                line3_h: z,
                line6: dec!(2000),
                line7: dec!(-2000),
                line8a_d: z,
                line8a_e: z,
                line8a_h: z,
                line10_d: z,
                line10_e: z,
                line10_h: z,
                line13: z,
                line14: z,
                line15: z,
                line16: dec!(-2000),
                routing: ScheduleDRouting::NetLoss {
                    line21: dec!(2000),
                    line22_yes: false,
                },
            },
            "the printed Schedule D, line by line"
        );

        assert_eq!(
            pf.f1040,
            crate::tax::printed::Form1040Lines {
                line1z: dec!(5000),
                line1a: dec!(5000),
                line2a: z,
                line2b: z,
                line3a: z,
                line3b: z,
                line7: dec!(-2000),
                line8: z,
                line9: dec!(3000),
                line10: z,
                line11: dec!(3000),
                line12: dec!(14600),
                line13: z,
                line14: dec!(14600),
                // ★★★ L15 — "Subtract line 14 from line 11. If zero or less, enter -0-." THE FLOOR.
                //     A lift that relaxed it would print −11,600 here.
                line15: z,
                line16: z,
                line17: z,
                line18: z,
                // ★ FR-1 — no dependents on this fixture, so Schedule 8812 line 12 is not
                //   provably "No" and 1040 line 19 is BLANK, not a sworn $0.
                line19: None,
                line20: z,
                line21: z,
                line22: z,
                line23: z,
                line24: z,
                line25a: z,
                line25b: z,
                line25c: z,
                line25d: z,
                line26: z,
                line31: z,
                line32: z,
                line33: z,
                line34: z,
                line37: z,
                digital_asset_yes: false,
            },
            "the printed 1040, line by line"
        );
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
            // D-8: an ordinary filer, who has ANSWERED that nobody can claim them.
            header: crate::tax::testonly::not_a_dependent(),
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

    /// Form 6251 wired through `screen_absolute`: a very-high-income filer whose AMTI clears the
    /// exemption by more than $232,600 still COMPUTES (line 7 lands at or below line 10), and so does a
    /// common household (Sch 2 line 2 = 0 for both).
    ///
    /// ★ Name and docstring both corrected here: they said "is REFUSED", describing the pre-Tier-1
    /// behaviour, while the body below had already been updated to assert the opposite. The screening
    /// worksheet is no longer what refuses anything.
    #[test]
    fn a_high_income_filer_and_a_common_household_both_compute_with_zero_amt() {
        let p = ty2024_params();
        let table = real_2024_table();
        // ★ BEHAVIOUR CHANGE (Tier 1). $900k wages trips the SCREEN (worksheet line 11 ≈ 887k >
        // 232,600) — but the screen only says "fill in Form 6251", so we fill it in. Line 7 lands
        // BELOW line 10, so no attachment is required (i6251 Who Must File, condition 1) and the
        // return COMPUTES. Before this task it was refused outright and no forms were written.
        let high = wages_single(dec!(900000));
        let ar_high = assemble_absolute(&high, &empty_ledger(), &p, &table, 2024);
        assert!(
            amt_should_file_6251(
                high.filing_status,
                ar_high.taxable_income,
                amt_worksheet_line2(
                    ar_high.deduction_is_itemized,
                    ar_high.standard_deduction,
                    Usd::ZERO
                ),
                Usd::ZERO,
                ar_high.regular_tax,
                Usd::ZERO,
                &p.amt,
            ),
            "fixture must still TRIP the cheap screen — otherwise it tests nothing"
        );
        assert!(
            ar_high.amt.line7 <= ar_high.amt.line10,
            "line 7 {} must not exceed line 10 {}",
            ar_high.amt.line7,
            ar_high.amt.line10
        );
        assert_eq!(ar_high.amt.amt(), Usd::ZERO, "no AMT is owed");
        assert!(
            !ar_high.amt.must_attach(),
            "no Form 6251 attachment required"
        );
        assert_eq!(
            screen_absolute(&high, &ar_high, &p, &empty_ledger(), 2024),
            None,
            "a screen-tripping, zero-AMT filer must now COMPUTE, not refuse"
        );
        // $150k wages → line 11 = 64,300 ≤ 232,600 and 26% × it < regular tax → cleared (no refuse).
        let common = wages_single(dec!(150000));
        let ar_common = assemble_absolute(&common, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            screen_absolute(&common, &ar_common, &p, &empty_ledger(), 2024),
            None
        );
    }

    /// ★ REGRESSION (whole-branch review, Critical) — a **Qualifying Surviving Spouse** full return must
    /// COMPUTE, not panic.
    ///
    /// `TaxTable::ltcg` is never populated with a `Qss` key — the adapters say so outright ("QSS is not
    /// inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time"), which is why
    /// [`TaxTable::ltcg_for`] exists and why the regular-tax call site uses it. Feeding Part III's
    /// §1(h) breakpoints via a raw `ltcg.get(&status).expect(..)` therefore aborts the process for every
    /// QSS filer — a status that is user-selectable in the CLI, the input form and the TUI, is refused
    /// nowhere, and computed fine before Form 6251 was wired in.
    ///
    /// Form 6251 lines 19 and 25 read "$94,050 if married filing jointly **or qualifying surviving
    /// spouse**" and "$583,750 if married filing jointly **or qualifying surviving spouse**", so the
    /// correct values were always available; only the lookup was wrong.
    #[test]
    fn a_qualifying_surviving_spouse_full_return_computes_rather_than_panicking() {
        let p = ty2024_params();
        let table = real_2024_table();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Qss,
            header: crate::tax::testonly::not_a_dependent(),
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(400000),
                dec!(168600),
                dec!(400000),
            )],
            // Preferential income, so line 7 routes into Part III and lines 19/25 are actually read.
            // Qualified dividends come straight from `ReturnInputs`, so no ledger is needed.
            div_1099: vec![crate::tax::return_inputs::Form1099Div {
                payer: "Broker".into(),
                box1a_ordinary: dec!(300000),
                box1b_qualified: dec!(300000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert!(
            ar.amt.line12 > Usd::ZERO,
            "fixture must reach Part III, else lines 19/25 are never read and this tests nothing"
        );
        assert_eq!(
            ar.amt.line19,
            dec!(94050),
            "QSS shares the MFJ 0%-band top (Form 6251 line 19)"
        );
        assert_eq!(
            ar.amt.line25,
            dec!(583750),
            "QSS shares the MFJ 15%-band top (Form 6251 line 25)"
        );
        assert_eq!(ar.amt.amt(), Usd::ZERO, "no AMT on these facts");
    }

    /// ★ T4 — a screen-tripping, NO-ATTACHMENT return produces a packet whose AMT lines are all $0.
    ///
    /// This is the assertion the plan wanted made against a hand-built expectation rather than by
    /// diffing against "what we produce today" — today produced nothing at all, so a self-comparison
    /// would have been vacuous. It pins the whole Tier-1 contract in one place: the cheap screen
    /// TRIPS, Form 6251 fills, line 7 lands at or below line 10, and therefore Schedule 2 line 2
    /// (→ 1040 L17) is $0 and no Form 6251 is attached.
    #[test]
    fn a_screen_tripping_no_attachment_return_reports_zero_amt_everywhere() {
        let p = ty2024_params();
        let table = real_2024_table();
        let ri = wages_single(dec!(900000));
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);

        // 1. The cheap screen must actually trip, or this fixture proves nothing.
        assert!(
            amt_should_file_6251(
                ri.filing_status,
                ar.taxable_income,
                amt_worksheet_line2(ar.deduction_is_itemized, ar.standard_deduction, Usd::ZERO),
                Usd::ZERO,
                ar.regular_tax,
                Usd::ZERO,
                &p.amt,
            ),
            "fixture must trip the screen"
        );
        // 2. Form 6251 fills, and the Who-Must-File comparison clears.
        assert!(ar.amt.line6 > Usd::ZERO, "AMTI exceeds the exemption");
        assert!(
            ar.amt.line7 <= ar.amt.line10,
            "line 7 must not exceed line 10"
        );
        assert!(!ar.amt.must_attach(), "no attachment required");
        // 3. Therefore every AMT-carrying line is zero.
        assert_eq!(ar.amt.line11, Usd::ZERO, "Form 6251 line 11");
        assert_eq!(ar.amt.amt(), Usd::ZERO, "→ Schedule 2 line 2");
        // 4. And the return computes rather than refusing.
        assert_eq!(screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024), None);
    }

    /// ★ REGRESSION (2026-07-27, `fix/amt-screen-line2`) — **an ITEMIZER must not have their non-SALT
    /// itemized deductions added back on worksheet line 2.**
    ///
    /// Worksheet line 2 is Schedule A line **7** (capped SALT), not line **17** (the itemized total).
    /// Mortgage interest, charitable gifts and medical are AMT-*allowed* under §56(b). The screen used to
    /// reduce the worksheet to `line 3 = AGI − QBI`, which silently added back ALL of them and refused
    /// ordinary itemizers.
    ///
    /// MFJ, $300,000 wages; $80,000 itemized = $10,000 real-estate tax (the whole §164(b)(6) cap) +
    /// $40,000 mortgage interest + $30,000 cash charity. Taxable income $220,000; regular tax $38,885.
    ///   CORRECT: line 3 = 220,000 + 10,000 = 230,000 → line 11 = 96,700 → 26% × it = 25,142 ≤ 38,885
    ///            → CLEARED, the return computes.
    ///   OLD BUG: line 3 = AGI 300,000 → line 11 = 166,700 → 26% × it = 43,342 > 38,885 → REFUSED.
    ///
    /// ★ WHAT THIS HOLDS — **measured, not asserted in prose** (whole-branch review I-1 found the original
    /// claim here to be a FALSE PASS: the mutation it named did not red it, because `screen_absolute`'s
    /// outcome no longer depends on the worksheet at all). Two parts, each with its verified killer:
    ///   1. **The worksheet fix.** `amt_worksheet_line2` returns Schedule A line **7** for an itemizer,
    ///      not the itemized total and not the standard deduction they did not take. **Verified
    ///      mutation:** make the `deduction_is_itemized` branch return `standard_deduction` and the
    ///      `dec!(10000)` assertion below reds — here and in `amt::tests::
    ///      itemizer_addback_is_schedule_a_line7_not_the_itemized_total`, the only two killers. (An
    ///      earlier draft of this docstring claimed to be the ONLY one; re-review measured two. Stated
    ///      as measured.) This is the only END-TO-END killer: the two `screen_absolute` cases above both
    ///      take the STANDARD deduction, the one branch where `L15 + std == AGI − QBI` makes the old
    ///      closed form accidentally correct. That is why this test exists.
    ///   2. **The user-visible guarantee** — this filer is not refused. Now delivered by
    ///      `ar.amt.must_attach()`, not by the worksheet. The `>` boundary in `must_attach` is held
    ///      broadly rather than here: **verified mutation** `line7 >= line10` reds 13 tests across the
    ///      workspace, including `a_cleared_screen_never_hides_a_must_attach_return` and the full-return
    ///      export suite. This fixture clears with margin, so it pins the OUTCOME, not the boundary —
    ///      stated explicitly so no future reader mistakes it for a boundary test.
    #[test]
    fn amt_screen_does_not_add_back_an_itemizers_mortgage_and_charity() {
        let p = ty2024_params();
        let table = real_2024_table();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            header: crate::tax::testonly::not_a_dependent(),
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(300000),
                dec!(168600),
                dec!(300000),
            )],
            schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                salt_real_estate: dec!(10000), // Sch A 5b → 5e capped at 10,000 → line 7
                mortgage_interest_1098: dec!(40000), // AMT-ALLOWED, must NOT be added back
                mortgage_all_used_to_buy_build_improve: Some(true),
                charitable: vec![crate::tax::return_inputs::CharitableGift {
                    class: crate::tax::return_inputs::CharitableClass::Cash60,
                    amount: dec!(30000), // AMT-ALLOWED, must NOT be added back
                }],
                ..Default::default()
            }),
            // §170(f)(8) neutral — a $30,000 gift is over $250, so the CWA gate is live on this
            // itemizer. Answered so the assertion below still measures the AMT screen.
            charitable_cwa_obtained: Some(true),
            ..Default::default()
        };

        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert!(
            ar.deduction_is_itemized,
            "fixture must actually itemize ($80,000 > the $29,200 MFJ standard) or it exercises the \
             wrong branch"
        );
        assert_eq!(ar.taxable_income, dec!(220000), "fixture: 1040 L15");
        assert_eq!(
            ar.schedule_a.as_ref().map(|s| s.salt_5e),
            Some(dec!(10000)),
            "fixture: Schedule A line 7 is the CAPPED SALT, and it is only an eighth of the itemized total"
        );
        // (1) The guarantee: no false refusal. This is the assertion a filer would feel.
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024),
            None,
            "an ordinary MFJ itemizer with $300k of wages owes no AMT and Form 6251 line 7 lands below \
             line 10; adding back the AMT-allowed mortgage/charitable deductions manufactures a false \
             refusal"
        );
        // …and it is the WHO-MUST-FILE test that delivers it, on the real form.
        assert!(
            ar.amt.line7 <= ar.amt.line10,
            "Who Must File condition 1 must clear on the real Form 6251"
        );

        // (2) The worksheet fix, asserted where it actually lives. $10,000 is Schedule A line 7 (the
        // whole §164(b)(6) cap); the itemized total is $80,000. Returning the latter is the bug.
        assert_eq!(
            amt_worksheet_line2(ar.deduction_is_itemized, ar.standard_deduction, dec!(10000)),
            dec!(10000),
            "worksheet line 2 for an itemizer is Schedule A line 7 — NOT the $80,000 itemized total, and \
             NOT the $29,200 standard deduction they did not take"
        );
        // The old closed form's line 3 (AGI − QBI = $300,000) against the correct one ($230,000): a
        // $70,000 gap, entirely AMT-allowed deductions.
        assert_eq!(
            ar.taxable_income + dec!(10000),
            dec!(230000),
            "correct worksheet line 3; the closed form said $300,000 and refused"
        );
    }

    /// ★ I-1 CROSS-CHECK — the `amt.rs` soundness claim, as a sweep: **a return the screening worksheet
    /// CLEARS never turns out to require Form 6251.**
    ///
    /// The screen is no longer control flow (`screen_absolute` reads the real form). It survives as a
    /// claim: the 1040 worksheet's line-12 test is a valid *sufficient* condition for "no AMT", because
    /// worksheet line 3 is AMTI exactly within v1's accepted inputs. If that were ever false, a future
    /// author restoring the screen as a fast path would silently skip attachable returns. This pins the
    /// implication `¬screen ⇒ ¬must_attach` across the grid rather than leaving it as prose.
    #[test]
    fn a_cleared_screen_never_hides_a_must_attach_return() {
        let p = ty2024_params();
        let table = real_2024_table();
        let mut checked = 0_u32;
        let mut cleared = 0_u32;
        // ★ Single and Mfj only — this module's `real_2024_table()` carries schedules for exactly those
        // two statuses (asserted below, so a widened table is a loud reminder to widen the grid). MFS and
        // HoH are covered against hand-derived figures by the `form6251.rs` vector suite, including the
        // §55(d)(3) MFS line-4 kicker.
        assert_eq!(table.ordinary.len(), 2, "grid must track the fixture table");
        for status in [FilingStatus::Single, FilingStatus::Mfj] {
            for wages in [
                dec!(0),
                dec!(50000),
                dec!(120000),
                dec!(200000),
                dec!(400000),
                dec!(900000),
                dec!(2000000),
            ] {
                // Both deduction branches: the standard one, and an itemizer whose Schedule A line 7 is a
                // small fraction of their itemized total (the branch the line-2 fix exists for).
                for itemized in [false, true] {
                    let mut ri = ReturnInputs {
                        filing_status: status,
                        ..wages_single(wages)
                    };
                    if itemized {
                        ri.schedule_a = Some(crate::tax::return_inputs::ScheduleAInputs {
                            salt_real_estate: dec!(10000),
                            mortgage_interest_1098: dec!(40000),
                            mortgage_all_used_to_buy_build_improve: Some(true),
                            mortgage_dwelling_is_amt_qualified: Some(true),
                            charitable: vec![crate::tax::return_inputs::CharitableGift {
                                class: crate::tax::return_inputs::CharitableClass::Cash60,
                                amount: dec!(30000),
                            }],
                            ..Default::default()
                        });
                    }
                    let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
                    let sch_a_line7 = ar.schedule_a.as_ref().map_or(Usd::ZERO, |x| x.salt_5e);
                    let screen_says_file = amt_should_file_6251(
                        ri.filing_status,
                        ar.taxable_income,
                        amt_worksheet_line2(
                            ar.deduction_is_itemized,
                            ar.standard_deduction,
                            sch_a_line7,
                        ),
                        ri.sch1.state_refund_taxable,
                        ar.regular_tax,
                        Usd::ZERO,
                        &p.amt,
                    );
                    checked += 1;
                    if !screen_says_file {
                        cleared += 1;
                        assert!(
                            !ar.amt.must_attach(),
                            "the worksheet CLEARED {status:?} @ {wages} wages (itemized={itemized}), \
                             but the real Form 6251 has line 7 ({}) > line 10 ({}) — the soundness \
                             claim in `amt.rs` is broken",
                            ar.amt.line7,
                            ar.amt.line10
                        );
                    }
                }
            }
        }
        // Both branches must be exercised, or the sweep is vacuous in one direction: 17 of the 28 grid
        // points clear the screen (the branch carrying the implication under test) and 11 trip it (the
        // branch proving the screen is not trivially always-clear).
        assert_eq!(checked, 28, "grid size");
        assert_eq!(cleared, 17, "cleared branch: the implication under test");
        assert_eq!(
            checked - cleared,
            11,
            "tripped branch: the screen is not vacuously clear"
        );
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
        let w2_ss = |owner: Owner, box4: Usd, ein: &str| W2 {
            owner,
            box4_ss_withheld: box4,
            ein: Some(ein.to_string()),
            ..Default::default()
        };
        // TWO employers, each $6,000 → Σ $12,000 > MAX → excess $1,546.80.
        // ★ This case previously passed with NO EIN on either W-2: the test's comment said "two
        //   employers" and its fixture never said so. The rule it was guarding — §6413(c)'s "more than
        //   one employer" — was therefore unasserted, which is exactly how the understatement shipped.
        let two = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(6000), "22-2222222"),
            ],
            ..Default::default()
        };
        assert_eq!(excess_social_security(&two, &table), dec!(1546.80));

        // ★★★ THE DEFECT, PINNED: the SAME two W-2s under ONE EIN credit NOTHING. i1040gi — "if you …
        //     had MORE THAN ONE EMPLOYER". One employer's over-withholding is recovered from the
        //     employer. A filing trial credited $3,894 here and turned an $1,085 liability into a
        //     $2,809 refund.
        let same_employer = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(6000), "11-1111111"),
            ],
            ..Default::default()
        };
        assert_eq!(
            excess_social_security(&same_employer, &table),
            Usd::ZERO,
            "one employer over several W-2s is NOT 'more than one employer' — crediting here \
             UNDERSTATES tax on a §6065 return"
        );

        // ★★★ THE SAME EMPLOYER, SPELLED TWO WAYS — a W-2 and its W-2c, box b typed off the paper
        //     form on one and off a payroll-portal export on the other. A Fable review built exactly
        //     this and got a $1,946.80 credit for a filer entitled to $0: identity was decided by a
        //     STRING COMPARE, which the field's own doc comment had warned against in so many words.
        let two_spellings = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(6000), "111111111"),
            ],
            ..Default::default()
        };
        assert_eq!(
            excess_social_security(&two_spellings, &table),
            Usd::ZERO,
            "an EIN's two standard renderings are ONE employer — comparing spellings instead of \
             identities restores the §6413(c) understatement"
        );

        // …and whitespace/format noise is identity-neutral in the other direction too.
        assert_eq!(canonical_ein(" 11-1111111 "), Some("111111111".to_string()));
        assert_eq!(
            canonical_ein("11-111111"),
            None,
            "eight digits is not an EIN"
        );
        assert_eq!(canonical_ein("XX-1111111"), None, "letters are not an EIN");

        // One employer $6,000 (< MAX) → no excess, and no EIN is needed to say so.
        let one = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box4_ss_withheld: dec!(6000),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(excess_social_security(&one, &table), Usd::ZERO);

        // ★★ "But if any ONE employer withheld more than $10,453.20, you can't claim the excess" —
        //    each EMPLOYER contributes at most the cap before the aggregate is compared to it. Employer
        //    A withheld $12,000 (over the cap on its own) and B $6,000: only $10,453.20 + $6,000 is
        //    creditable, so the credit is $6,000, NOT the naive $18,000 − $10,453.20 = $7,546.80.
        let one_over_cap = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(12000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(6000), "22-2222222"),
            ],
            ..Default::default()
        };
        assert_eq!(
            excess_social_security(&one_over_cap, &table),
            dec!(6000),
            "an employer's own over-cap withholding is not claimable on the return"
        );

        // ★★★ r8 I-1 — THE DISCLOSURE IS PER EMPLOYER, NOT PER PERSON. The first version fired only
        //     when a person had EXACTLY ONE employer, which is a different test from the instruction's
        //     *"if ANY ONE EMPLOYER withheld more than $10,453.20"*. The reviewer's sharpest probe:
        //     adding a second employer who withheld NOTHING left the tax outcome byte-identical and
        //     switched the disclosure OFF.
        let one_employer_over = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2_ss(Owner::Taxpayer, dec!(12000), "11-1111111")],
            ..Default::default()
        };
        let with_a_silent_second = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(12000), "11-1111111"),
                w2_ss(Owner::Taxpayer, Usd::ZERO, "44-4444444"),
            ],
            ..Default::default()
        };
        for ri in [&one_employer_over, &with_a_silent_second] {
            assert_eq!(excess_social_security(ri, &table), Usd::ZERO);
            let nc = non_creditable_ss(ri, &table);
            assert_eq!(
                nc.len(),
                1,
                "one employer over the cap ⇒ one disclosure: {nc:?}"
            );
            assert_eq!(nc[0].amount, dec!(1546.80));
            assert_eq!(nc[0].ein, "111111111");
        }

        // …and it fires ALONGSIDE a correct credit. Employer A strands $546.80 while the return
        // rightly pays $2,000 on B's contribution — both facts are true at once.
        let over_and_credited = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(11000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(2000), "44-4444444"),
            ],
            ..Default::default()
        };
        assert_eq!(
            excess_social_security(&over_and_credited, &table),
            dec!(2000)
        );
        let nc = non_creditable_ss(&over_and_credited, &table);
        assert_eq!((nc.len(), nc[0].amount), (1, dec!(546.80)));

        // ★★ r8 I-2 — NEVER POOLED. Two spouses, two employers, two disclosures: summing them yields a
        //    number no employer withheld, attached to a message saying "ask THAT employer".
        let mfj_both_over = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(12000), "11-1111111"),
                w2_ss(Owner::Spouse, dec!(11000), "33-3333333"),
            ],
            ..Default::default()
        };
        let nc = non_creditable_ss(&mfj_both_over, &table);
        assert_eq!(nc.len(), 2, "one disclosure per (person, employer): {nc:?}");
        assert_eq!(nc[0].amount, dec!(1546.80));
        assert_eq!(nc[1].amount, dec!(546.80));
        assert_ne!(
            nc[0].owner, nc[1].owner,
            "the two disclosures belong to DIFFERENT people — pooling them invents an employer"
        );

        // MFJ: taxpayer 2×$6,000 across two EINs (excess $1,546.80) + spouse 1×$8,000 (< MAX → 0) →
        // total $1,546.80, NOT the pooled max(0, 20,000 − 10,453.20) = $9,546.80.
        let mfj = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2_ss(Owner::Taxpayer, dec!(6000), "11-1111111"),
                w2_ss(Owner::Taxpayer, dec!(6000), "22-2222222"),
                w2_ss(Owner::Spouse, dec!(8000), "33-3333333"),
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
            // D-8: a retiree, not somebody's dependent — and the return must SAY so (an unanswered flag
            // now conservatively takes the §63(c)(5) floor, which would change this test's numbers).
            header: crate::tax::testonly::not_a_dependent(),
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
    /// A current-year `ReturnInputs` with the §G-21 restriction question ANSWERED NO, so the
    /// write-back tests below exercise the guard they were each written for rather than tripping the
    /// vouch-for gate. The gate itself is pinned by
    /// `a_carryover_btctax_cannot_vouch_for_is_never_written_into_next_year`.
    fn plain_ri() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Single,
            donations_had_restrictions: Some(false),
            ..Default::default()
        }
    }

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
                qbi_carryforward_in: Usd::ZERO,
                qbi_carryforward_in_provenance: CarryProvenance::User,
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

    /// ★★★ Form 8995 **line 16** must reach next year's line 3, or the loss is EXTINGUISHED.
    ///
    /// This test exists because dropping the write-back line was found to be INVISIBLE: the whole
    /// suite stayed green with `next_year.qbi.qbi_carryforward_in = ar.qbi_carryforward_out;` deleted.
    /// A lost carryforward is not a one-year error — it silently hands the filer a larger deduction in
    /// every later year than they are entitled to, and no single year's return looks wrong.
    ///
    /// $10,000 of QBI against a $30,000 prior-year loss: line 4 floors at -0-, and line 16 carries the
    /// unabsorbed $20,000 forward, stamped `Computed` so a later report may overwrite it silently.
    #[test]
    fn form_8995_line16_carries_into_next_years_line3() {
        let mut ri = crate::tax::testonly::answered(ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            ..Default::default()
        });
        ri.header.taxpayer = crate::tax::return_inputs::Person {
            first_name: "John".into(),
            last_name: "Doe".into(),
            ssn: "123456789".into(),
            ..Default::default()
        };
        ri.w2s.push(W2 {
            box1_wages: dec!(80000),
            ..Default::default()
        });
        ri.qbi.qbi_carryforward_in = dec!(30000);
        ri.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
            business_description: "Bitcoin mining".into(),
            ..Default::default()
        });

        let ar = assemble_absolute(
            &ri,
            &Default::default(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        assert!(
            ar.qbi_carryforward_out > Usd::ZERO,
            "an unabsorbed prior-year QBI loss must carry OUT (Form 8995 line 16), got {}",
            ar.qbi_carryforward_out
        );

        let next = apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            next.qbi.qbi_carryforward_in, ar.qbi_carryforward_out,
            "★ line 16 must land on NEXT year's line 3 — dropping this silently extinguishes the loss"
        );
        assert_eq!(
            next.qbi.qbi_carryforward_in_provenance,
            CarryProvenance::Computed,
            "stamped Computed, so a later report may overwrite it without --force"
        );

        // …and a USER-entered carryforward is not silently overwritten, exactly like its REIT sibling.
        let mut user = ReturnInputs::default();
        user.qbi.qbi_carryforward_in = dec!(999);
        user.qbi.qbi_carryforward_in_provenance = CarryProvenance::User;
        assert!(
            apply_carryover_writeback(&ar, &plain_ri(), &empty_ledger(), 2024, user.clone(), false)
                .is_err(),
            "a user-entered business-loss carryforward needs --force to overwrite"
        );
        assert!(
            apply_carryover_writeback(&ar, &plain_ri(), &empty_ledger(), 2024, user, true).is_ok()
        );
    }

    /// ★★★ **FOLD-REVIEW — r3's I-3 fix swapped one wrong predicate for ANOTHER, and I wrote the
    /// standard it violates three lines above it.**
    ///
    /// r3 correctly diagnosed that the gate read the LEDGER rather than the RETURN, and its own
    /// rationale says so verbatim: *"TOO WIDE. `year_donation_deduction` reads the LEDGER, not the
    /// return … a restriction changes no figure — yet they were refused, unescapably, by a message
    /// asserting 'this year files a Form 8283 SECTION B'. It does not."* Then it re-keyed to
    /// `ar.deduction_is_itemized && year_donation_deduction(state, year) > 0` — which reads the return
    /// for the ELECTION and **still reads the ledger for the AMOUNT**. Same organ, half-treated.
    ///
    /// The case that survives it: an itemizing filer whose §170(b) ceiling zeroes the noncash
    /// deduction. AGI $0 with $20,000 of mortgage interest itemizes on the mortgage alone, while
    /// 30% × $0 = $0 allows no charitable deduction at all. Schedule A line 12 is `0`, the packet
    /// writes **no Form 8283** — both skeptics exported it and confirmed the packet holds only
    /// `00_f1040.pdf`, `07_f1040sa.pdf` and `manifest.txt` — and the return was still hard-blocked.
    /// The only exits were a false "No" under §6065, or deleting a truthful ledger event.
    ///
    /// ★★ The predicate is now the quantity `packet.rs` ITSELF filters on — Schedule A line 12, the
    /// §170(b)-LIMITED figure — so the gate and the packet cannot disagree about whether an 8283
    /// exists. Form 8283's own text keys on the same thing (f8283--2025.txt:8,10: *"Attach one or more
    /// Forms 8283 to your tax return if you claimed a total deduction of over $500 for all contributed
    /// property"* — the CLAIMED deduction, not the ledger's fair market value).
    ///
    /// ★ The ceiling-zeroed year is not thereby unguarded: its excess rolls forward, and
    /// `apply_carryover_writeback`'s vouch-for gate refuses to persist it. The year files clean
    /// because it claims nothing; the carryover cannot be laundered. Those two gates are the pair.
    ///
    /// Mutation-verified: restoring either `year_donation_deduction`-keyed arm reds the ceiling row.
    #[test]
    fn an_itemizer_whose_170b_ceiling_zeroes_the_gift_files_no_8283_and_is_not_blocked() {
        let p = ty2024_params();
        let table = real_2024_table();
        // AGI $0. Mortgage interest alone ($20,000) beats the $14,600 standard, so the return
        // ITEMIZES — but §170(b)'s 30% ceiling on a $0 base allows $0 of noncash charity.
        let screened = |answer: Option<bool>, claimed: Usd| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                // ★ Answered so this test isolates its SUBJECT — the restrictions gate. Since the
                //   phase-2 review's R3 fold, a ceiling-zeroed year is inside the §170(f)(8) gate
                //   too (the claim is DEFERRED under §170(d), not denied, while the acknowledgment
                //   deadline still dies at this filing). Leaving it unanswered would make every row
                //   below refuse for the other gate's reason and prove nothing about this one.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let st = state_removals(vec![donation(
                date!(2024 - 06 - 01),
                vec![donation_leg(Term::LongTerm, dec!(1000), claimed)],
            )]);
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            assert!(ar.deduction_is_itemized, "fixture must ITEMIZE");
            assert_eq!(
                ar.schedule_a
                    .as_ref()
                    .map_or(Usd::ZERO, |a| a.charitable_noncash_12),
                Usd::ZERO,
                "…and the §170(b) ceiling must zero the noncash deduction — the whole point"
            );
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };

        // ★ THE DEFECT: the return claims $0 of noncash charity and attaches no 8283, so a
        //   restriction moves no figure — but the ledger-keyed gate refused it, escapably only by
        //   perjury or by deleting a truthful event.
        assert_eq!(
            screened(Some(true), dec!(4000)),
            None,
            "nothing is claimed, so nothing is overstated — refusing here is a false block"
        );
        assert_eq!(
            screened(None, dec!(50000)),
            None,
            "…and the unanswered arm asserted 'this year files a Form 8283 SECTION B'. It does not."
        );

        // ★★ AND THE BAND BETWEEN THEM. A ceiling can land the claimed deduction ABOVE $0 but at or
        //    below the $500 attachment threshold — AGI $1,600 gives a 30% ceiling of $480 — so the
        //    return claims a noncash deduction and STILL files no Form 8283. The unanswered arm must
        //    not fire there: with no 8283 attached, lines 5a/5b/5c are never printed, so silence
        //    asserts nothing. (Mutation-verified: dropping the `> FORM_8283_THRESHOLD` term reds this.)
        let band = |answer: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                // §170(f)(8) neutral — the $50,000 contribution is over $250, so the CWA gate IS
                // live here; this test is about the §G-21 8283-attachment band, not about it.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                w2s: vec![w2(Owner::Taxpayer, dec!(1600), dec!(1600), dec!(1600))],
                ..Default::default()
            };
            let st = state_removals(vec![donation(
                date!(2024 - 06 - 01),
                vec![donation_leg(Term::LongTerm, dec!(1000), dec!(50000))],
            )]);
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            let l12 = ar
                .schedule_a
                .as_ref()
                .map_or(Usd::ZERO, |a| a.charitable_noncash_12);
            assert!(
                l12 > Usd::ZERO && l12 <= crate::tax::printed::FORM_8283_THRESHOLD,
                "fixture must claim a noncash deduction in the no-8283 band, got {l12}"
            );
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };
        assert_eq!(
            band(None),
            None,
            "no 8283 attaches, so 5a/5b/5c are never printed — an unanswered filer asserts nothing"
        );
        // …but a DECLARED restriction still refuses, because the claimed deduction is real and too
        // large whatever the form-attachment threshold says. Reg §1.170A-7 is about the DEDUCTION.
        assert_eq!(
            band(Some(true)),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "the $480 it claims is still overstated — the 8283 threshold governs paperwork, not §170"
        );

        // …while a return that DOES claim the deduction is still caught. $60,000 of AGI gives a
        // $18,000 ceiling, so line 12 is non-zero and an 8283 really does attach.
        let claiming = |answer: Option<bool>, claimed: Usd| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                // §170(f)(8) neutral — this test is about the §G-21 restriction question.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                w2s: vec![w2(Owner::Taxpayer, dec!(60000), dec!(60000), dec!(60000))],
                ..Default::default()
            };
            let st = state_removals(vec![donation(
                date!(2024 - 06 - 01),
                vec![donation_leg(Term::LongTerm, dec!(1000), claimed)],
            )]);
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            assert!(
                ar.schedule_a
                    .as_ref()
                    .map_or(Usd::ZERO, |a| a.charitable_noncash_12)
                    > crate::tax::printed::FORM_8283_THRESHOLD,
                "fixture must actually claim a noncash deduction over $500"
            );
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };
        assert_eq!(
            claiming(Some(true), dec!(9000)),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "a claimed, restricted deduction is still too large — refuse"
        );
        assert_eq!(
            claiming(None, dec!(9000)),
            Some(RefuseReason::DonationRestrictionsUnresolved),
            "…and an attached Section B still prints 5a/5b/5c, which btctax may not answer"
        );
    }

    /// ★★★ **P4 / §170(f)(8) — THE CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT TERNARY.**
    ///
    /// unanswered ⇒ refuse, `Some(false)` ⇒ refuse, `Some(true)` ⇒ proceed — on a return that
    /// itemizes and claims a §170 deduction with at least one single gift of $250 or more.
    ///
    /// **B1 mutations, each observed RED before the fix landed:**
    /// - delete the `None` arm ⇒ the unanswered row reds with `None`: btctax silently claims a
    ///   deduction §170(f)(8)(A) may deny, and the filer's chance to cure it dies at filing;
    /// - delete the `Some(false)` arm ⇒ the "no acknowledgment" row reds the same way, which is the
    ///   worse half — the filer TOLD us and we filed it anyway.
    #[test]
    fn the_cwa_question_refuses_unanswered_and_refuses_no_but_proceeds_on_yes() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let screened = |answer: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: Some(false),
                charitable_cwa_obtained: answer,
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    salt_state_estimated_payments: dec!(10000),
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                w2s: vec![w2(
                    Owner::Taxpayer,
                    dec!(200000),
                    dec!(168600),
                    dec!(200000),
                )],
                ..Default::default()
            };
            let st = donation_state(dec!(4000));
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            assert!(ar.deduction_is_itemized, "the fixture must itemize");
            screen_absolute(&ri, &ar, &p, &st, 2024).map(|r| r.reason)
        };
        assert_eq!(
            screened(None),
            Some(RefuseReason::CharitableCwaUnresolved),
            "unanswered ⇒ refuse: §170(f)(8)(A) conditions the deduction on the acknowledgment, and \
             filing is the moment the cure expires"
        );
        assert_eq!(
            screened(Some(false)),
            Some(RefuseReason::CharitableCwaUnresolved),
            "\"no, I don't hold one\" ⇒ refuse: the deduction as computed is disallowed by statute"
        );
        assert_eq!(screened(Some(true)), None, "holding one ⇒ file");
    }

    /// ★★★ **P4 — THE SCOPING, which is how the owner's \"too aggressive with refusals\" correction is
    /// honoured STRUCTURALLY rather than by wording.** Three filers must NEVER be asked, and each
    /// failing row is a real over-refusal a live filer would hit:
    ///
    /// 1. **the standard-deduction filer.** §170(f)(8) conditions a *deduction*; they claim none, so
    ///    silence forgoes nothing and asserts nothing.
    /// 2. **the filer whose every gift is under $250.** i1040sca: *"In figuring whether a gift is $250
    ///    or more, don't combine separate donations."* PER CONTRIBUTION — a year AGGREGATE of $400
    ///    across two $200 gifts poses no question.
    /// 3. **the itemizer whose §170(b) ceiling zeroes the charitable deduction entirely** — they claim
    ///    nothing either, so there is nothing for the statute to disallow.
    ///
    /// **B1 mutations, each observed RED:**
    /// - drop the `deduction_is_itemized` conjunct ⇒ row 1 reds;
    /// - key the threshold on the year AGGREGATE (`year_donation_deduction`) instead of the largest
    ///   SINGLE contribution ⇒ row 2 reds — the exact defect the adjudication names;
    /// - drop the `cwa_claimed > 0` conjunct ⇒ row 3 reds.
    #[test]
    fn the_cwa_question_is_never_posed_to_a_standard_deduction_or_small_gift_filer() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let run = |sched_a: Option<crate::tax::return_inputs::ScheduleAInputs>,
                   wages: Usd,
                   st: LedgerState| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: Some(false),
                charitable_cwa_obtained: None, // NEVER ANSWERED — the whole point
                schedule_a: sched_a,
                w2s: vec![w2(Owner::Taxpayer, wages, wages, wages)],
                ..Default::default()
            };
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            let r = screen_absolute(&ri, &ar, &p, &st, 2024);
            (
                ar.deduction_is_itemized,
                r.as_ref().map(|r| r.reason.clone()),
                r.map(|r| r.detail).unwrap_or_default(),
            )
        };

        // ★★★ (1) THE STANDARD-DEDUCTION FILER, and the fixture has to be built with care or it
        //     proves nothing. A return with NO `schedule_a` at all leaves `ar.schedule_a` = `None`,
        //     so lines 11/12 are $0 and the gate is already shut by the CLAIMED-amount conjunct —
        //     dropping `deduction_is_itemized` would then survive, and did (B1 caught it). This
        //     fixture therefore HAS a Schedule A carrying a $5,000 cash gift — so
        //     `charitable_cash_11` is genuinely non-zero — but a $5,000 itemized total loses to the
        //     $14,600 standard deduction, which makes `deduction_is_itemized` the ONLY thing
        //     standing between this filer and a refusal.
        use crate::tax::return_inputs::{CharitableClass, CharitableGift};
        let loses_to_standard = crate::tax::return_inputs::ScheduleAInputs {
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: dec!(5000),
            }],
            ..Default::default()
        };
        let (itemized, reason, _detail) =
            run(Some(loses_to_standard), dec!(200000), empty_ledger());
        assert!(!itemized, "fixture 1 must take the standard deduction");
        assert_eq!(
            reason, None,
            "a standard-deduction filer claims no §170 deduction, so §170(f)(8) conditions nothing"
        );

        // (2) An ITEMIZER whose gifts are all under $250 — two $200 cash gifts, a $400 aggregate.
        //     Per contribution, so the question is never posed.
        let small = crate::tax::return_inputs::ScheduleAInputs {
            salt_state_estimated_payments: dec!(10000),
            mortgage_interest_1098: dec!(20000),
            charitable: vec![
                CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: dec!(200),
                },
                CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: dec!(200),
                },
            ],
            ..Default::default()
        };
        let (itemized, reason, _detail) = run(Some(small), dec!(200000), empty_ledger());
        assert!(itemized, "fixture 2 must itemize");
        assert_eq!(
            reason, None,
            "two $200 gifts are two contributions, not one $400 one — i1040sca says not to combine \
             them, so no CWA question exists"
        );

        // ★★★ (3) THE CEILING-ZEROED ITEMIZER — THIS ROW WAS INVERTED BY THE PHASE-2 REVIEW (R3).
        //
        //     It used to assert `None`, on the reasoning that "the §170(b) ceiling allows $0, so
        //     nothing is claimed and nothing can be disallowed". That conflates two zeros which
        //     differ in kind:
        //       - a §170(e)-reduced claim of $0 is EXTINGUISHED forever. No acknowledgment needed,
        //         and the per-item `claimed_deduction > 0` filter still excuses it (case 4 below).
        //       - a §170(b)-CEILING zero is DEFERRED. §170(d) carries the claim into the next five
        //         years, this engine computes the carryover-out, and P6 tells the filer to roll it.
        //         The deduction WILL be taken — just later.
        //
        //     Meanwhile §170(f)(8)(C) fixes the acknowledgment deadline at the earlier of filing or
        //     the due date OF THE CONTRIBUTION YEAR. So the cure dies at THIS filing while the claim
        //     lives on: this filer would file unasked, lose the acknowledgment permanently, and then
        //     deduct $50,000 across later years unsubstantiated. That is D4's own prong (3) — the
        //     silently lost right — reappearing on the gate built to close it.
        let zeroed = crate::tax::return_inputs::ScheduleAInputs {
            mortgage_interest_1098: dec!(20000),
            ..Default::default()
        };
        let (itemized, reason, detail) = run(Some(zeroed), Usd::ZERO, donation_state(dec!(50000)));
        assert!(
            itemized,
            "fixture 3 must itemize on the mortgage interest alone"
        );
        assert_eq!(
            reason,
            Some(RefuseReason::CharitableCwaUnresolved),
            "★ a ceiling-zeroed year DEFERS the claim under §170(d) but the §170(f)(8)(C) deadline \
             still dies at this filing — ask now or the cure is gone. MUTATION: drop the \
             `cwa_deferred_to_carryover` disjunct in `screen_absolute` and this reds."
        );
        // ★★★ AND THE WORDS MATCH THE POPULATION (final whole-branch review, finding 2).
        //
        // The R3 fold widened the gate's FIRING CONDITION to deferred claims and left the text
        // scoped to gifts "you are deducting this year". This filer's lines 11 and 12 are $0, so a
        // refusal opening "this return claims a charitable deduction" states something they can see
        // is untrue of their own return — and the same words in the QUESTION let them answer yes
        // with perfect honesty while holding nothing, defeating the gate that had just stopped them.
        assert!(
            !detail.contains("this return claims a charitable deduction"),
            "the refusal must not tell a ceiling-zeroed filer their return CLAIMS a deduction — \
             lines 11 and 12 are $0. Got: {detail}"
        );
        assert!(
            detail.contains("exceeded their §170(b)")
                && detail.contains("carries it forward")
                && detail.contains("you will deduct it in a later year"),
            "…it must say what is actually true of them: the claim is DEFERRED, not absent. \
             Got: {detail}"
        );
        assert!(
            detail.contains("runs from the return for the year of the CONTRIBUTION"),
            "…and name why the deadline is THIS return even though the deduction is later — that is \
             the one fact a deferral filer has no way to guess. Got: {detail}"
        );

        // ── The `Some(false)` arm's CURE must differ too: "remove that gift from the deduction" is
        //    meaningless to a filer whose return deducts nothing this year. ────────────────────────
        let zeroed_no = crate::tax::return_inputs::ScheduleAInputs {
            mortgage_interest_1098: dec!(20000),
            ..Default::default()
        };
        let ri_no = ReturnInputs {
            filing_status: FilingStatus::Single,
            donations_had_restrictions: Some(false),
            charitable_cwa_obtained: Some(false),
            schedule_a: Some(zeroed_no),
            ..Default::default()
        };
        let st_no = donation_state(dec!(50000));
        let ar_no = assemble_absolute(&ri_no, &st_no, &p, &table, 2024);
        let d = screen_absolute(&ri_no, &ar_no, &p, &st_no, 2024)
            .expect("answering NO on a deferred claim still refuses")
            .detail;
        assert!(
            !d.contains("remove that gift from the deduction"),
            "a filer deducting nothing this year has no deduction to remove — offering that cure is \
             the defect, not a wording nit. Got: {d}"
        );
        assert!(
            d.contains("not deductible in any year") && d.contains("must not be carried forward"),
            "…the honest cure is that §170(f)(8)(A) denies the gift outright, so nothing carries. \
             Got: {d}"
        );
    }

    // ── P8 — Form 8960 Part II line 9b: collect-or-blank, bounded by §164(b)(6) ──────────────────

    /// The P8 fixture: Single, $200,000 wages + `interest` of taxable interest, itemizing on
    /// $20,000 of mortgage interest, with `salt` of state income tax paid as estimates. AGI clears
    /// the $200,000 §1411 threshold, so a Form 8960 exists and line 9b is live.
    fn p8_return(salt: Usd, sales_tax: Option<Usd>, line9b: Option<Usd>) -> ReturnInputs {
        let mut a = crate::tax::return_inputs::ScheduleAInputs {
            salt_state_estimated_payments: salt,
            mortgage_interest_1098: dec!(20000),
            mortgage_all_used_to_buy_build_improve: Some(true),
            mortgage_within_debt_limit: Some(true),
            mortgage_dwelling_is_amt_qualified: Some(true),
            ..Default::default()
        };
        if let Some(amount) = sales_tax {
            a.salt_use_sales_tax = Some(true);
            a.salt_sales_tax_amount = amount;
        } else {
            a.salt_use_sales_tax = Some(false);
        }
        ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(200000),
                dec!(168600),
                dec!(200000),
            )],
            int_1099: vec![crate::tax::return_inputs::Form1099Int {
                payer: "Bank".into(),
                box1_interest: dec!(60000),
                ..Default::default()
            }],
            schedule_a: Some(a),
            form_8960_line9b: line9b,
            ..Default::default()
        }
    }

    /// ★★★ **B1 kill-test (a) — a collected Form 8960 line 9b ABOVE what §164(b)(6) let the return
    /// deduct is REFUSED.**
    ///
    /// §1411(c)(1)(B) allocates only *"the deductions **allowed by this subtitle**"*, and
    /// §164(b)(6)(B) caps the SALT taken into account at $10,000 — so of the $30,000 of state income
    /// tax this filer PAID, only $10,000 is a deduction at all, and only that $10,000 is allocable.
    /// A 9b of $10,001 has no source in the return.
    ///
    /// ★ The AT-THE-BOUND case is asserted in the same test on purpose: a gate that refused every
    /// value would pass the over-cap half while being useless, and this is the pair that tells them
    /// apart. Planting `if claimed_9b > bound` → `if false` (or deleting the block) reds the first
    /// assertion; making it `>=` reds the second.
    #[test]
    fn a_line9b_above_the_salt_cap_is_refused_and_one_at_the_bound_is_accepted() {
        let p = ty2024_params();
        let table = synthetic_table(2024);

        let over = p8_return(dec!(30000), None, Some(dec!(10001)));
        let ar = assemble_absolute(&over, &empty_ledger(), &p, &table, 2024);
        assert!(
            ar.deduction_is_itemized,
            "the fixture must ITEMIZE, else it proves the standard-deduction branch instead"
        );
        assert_eq!(
            ar.schedule_a.as_ref().unwrap().salt_5a,
            dec!(30000),
            "line 5a must exceed the cap, else the cap binds on nothing and the test is vacuous"
        );
        assert_eq!(
            nii_line9b_bound(&ar),
            dec!(10000),
            "the bound is min(5a, 5e, cap) = min(30,000, 10,000, 10,000)"
        );
        let r = screen_absolute(&over, &ar, &p, &empty_ledger(), 2024)
            .expect("$10,001 exceeds the $10,000 the return actually deducted");
        assert_eq!(r.reason, RefuseReason::Nii9bExceedsDeductedSalt);
        for phrase in ["§1411(c)(1)(B)", "§164(b)(6)(B)", "$10,000"] {
            assert!(
                r.detail.contains(phrase),
                "the refusal must cite {phrase:?}; got: {}",
                r.detail
            );
        }

        // …and a dollar less — exactly the pool the return deducted — files.
        let at = p8_return(dec!(30000), None, Some(dec!(10000)));
        let ar_at = assemble_absolute(&at, &empty_ledger(), &p, &table, 2024);
        assert_eq!(
            screen_absolute(&at, &ar_at, &p, &empty_ledger(), 2024).map(|r| r.reason),
            None,
            "a 9b AT the bound is the filer's own lawful allocation and must not be refused"
        );
    }

    /// ★★★ **B1 kill-test (b) — the §164(b)(5) SALES-TAX election bounds line 9b to $0.**
    ///
    /// i8960, *Line 9b*: *"Sales taxes aren't deductible in computing net investment income."* This
    /// is a SEPARATE mechanism from the cap, and the over-cap test above cannot cover it: under the
    /// election Schedule A line 5e is still $10,000 (of sales tax), so a bound computed from 5e — or
    /// from the cap — would happily admit $10,000 of a deduction §1411 does not allow at all. Only
    /// the `salt_is_sales_tax` branch of `nii_line9b_bound` catches it, and deleting that branch reds
    /// exactly here while leaving the over-cap test green.
    #[test]
    fn the_sales_tax_election_bounds_line9b_to_zero() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = p8_return(dec!(30000), Some(dec!(30000)), Some(dec!(1)));
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        let a = ar.schedule_a.as_ref().unwrap();
        assert!(a.salt_is_sales_tax, "the fixture must make the election");
        assert_eq!(
            a.salt_5e,
            dec!(10000),
            "5e must still be a full $10,000 of SALES tax — that is what makes a 5e-based bound wrong"
        );
        assert_eq!(
            nii_line9b_bound(&ar),
            Usd::ZERO,
            "sales taxes are never deductible in computing net investment income"
        );
        let r = screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024)
            .expect("even $1 of line 9b is unallowable under the sales-tax election");
        assert_eq!(r.reason, RefuseReason::Nii9bExceedsDeductedSalt);
        assert!(
            r.detail.contains("Sales taxes aren't deductible"),
            "the refusal must quote i8960's own sentence; got: {}",
            r.detail
        );
    }

    /// ★★ The STANDARD-deduction branch: nothing was *"properly deducted on your return"*, so the
    /// bound is $0 whatever the filer paid in state tax. Same Schedule A amounts, but with the
    /// mortgage interest removed so the standard deduction wins.
    #[test]
    fn a_standard_deduction_return_may_allocate_no_state_tax_to_nii() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let mut ri = p8_return(dec!(30000), None, Some(dec!(1)));
        ri.schedule_a.as_mut().unwrap().mortgage_interest_1098 = Usd::ZERO;
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert!(
            !ar.deduction_is_itemized,
            "the fixture must take the STANDARD deduction ($10,000 SALT < $14,600), else it is the \
             itemized branch again"
        );
        assert_eq!(nii_line9b_bound(&ar), Usd::ZERO);
        let r = screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024)
            .expect("a standard-deduction return deducted no state income tax to allocate");
        assert_eq!(r.reason, RefuseReason::Nii9bExceedsDeductedSalt);
        assert!(
            r.detail.contains("STANDARD deduction"),
            "the message must name the branch the filer is actually on; got: {}",
            r.detail
        );
    }

    /// ★★★ **BLANK STAYS BLANK, AND IT IS THE DEFAULT.** An unanswered line 9b must (a) never
    /// refuse — this is collect-**or-blank**, not a declaration — and (b) reach the printed chain as
    /// `None`, not as a computed zero. The two provenances are indistinguishable on the page, so the
    /// only place the distinction can be held is the type.
    ///
    /// ★ It also pins the negative half of the answered-ness invariant: btctax must NOT compute the
    /// allocation. If some future edit derived 9b from the ratio, `f8960.line9b` would arrive
    /// `Some(..)` on a return where the filer said nothing, and this reds.
    #[test]
    fn an_unanswered_line9b_neither_refuses_nor_prints_a_zero() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = p8_return(dec!(30000), None, None);
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert!(
            nii_line9b_bound(&ar) > Usd::ZERO,
            "there IS a pool to forgo"
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &empty_ledger(), 2024),
            None,
            "collect-or-blank: silence claims nothing and must never gate the return"
        );
        assert_eq!(ar.printed_inputs.form_8960_line9b, None);
        let f8960 = crate::tax::other_taxes::form_8960_lines(
            FilingStatus::Single,
            ar.taxable_interest,
            ar.ordinary_dividends,
            ar.capital_gain,
            ar.printed_inputs.crypto_lending_interest,
            ar.agi,
            ar.printed_inputs.form_8960_line9b,
        )
        .expect("NIIT is owed");
        assert_eq!(
            f8960.line9b, None,
            "line 9b must stay BLANK — a computed 0 would swear the allocable state tax IS zero"
        );
        assert_eq!(
            f8960.line9d,
            Usd::ZERO,
            "9d = 9a + 9b + 9c over three blanks"
        );
    }

    /// ★★ **SPEC §3.4 — the forgone Part II deduction gets a LOUD advisory, and only where something
    /// is actually forgone.** The mandate is "never forgo a benefit in silence"; it is not "nag".
    ///
    /// Four cells, one predicate: fires on (blank 9b ∧ NIIT owed ∧ bound > 0), silent on each of the
    /// three ways that fails. Dropping the `bound > Usd::ZERO` guard reds the sales-tax case;
    /// dropping `is_none()` reds the answered case; dropping `niit.tax > 0` reds the no-NIIT case.
    #[test]
    fn the_forgone_line9b_deduction_is_advised_exactly_where_it_exists() {
        use crate::tax::advisories::Advisory;
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let fires = |ri: &ReturnInputs| {
            let ar = assemble_absolute(ri, &empty_ledger(), &p, &table, 2024);
            crate::tax::advisories::advisories_for(ri, &empty_ledger(), &ar, &p, 2024)
                .into_iter()
                .find_map(|a| match a {
                    Advisory::Form8960Line9bNotClaimed { bound, saving } => Some((bound, saving)),
                    _ => None,
                })
        };

        // (1) Blank 9b, NIIT owed, a real $10,000 pool ⇒ fires, naming the pool AND the saving.
        //     This fixture is wages $200,000 + interest $60,000, so line 12 ($60,000) equals line 15
        //     ($260,000 AGI − $200,000) and the whole $10,000 allocation moves line 16 dollar for
        //     dollar: 3.8% × $10,000 = $380.
        assert_eq!(
            fires(&p8_return(dec!(30000), None, None)),
            Some((dec!(10000), dec!(380)))
        );

        // (2) The filer ANSWERED — nothing is forgone in silence, so nothing is said.
        assert_eq!(fires(&p8_return(dec!(30000), None, Some(dec!(4000)))), None);

        // (3) The §164(b)(5) sales-tax election ⇒ the pool is $0 and there is nothing to forgo.
        assert_eq!(
            fires(&p8_return(dec!(30000), Some(dec!(30000)), None)),
            None
        );

        // (4) No NIIT owed (drop the interest ⇒ MAGI under the $200,000 threshold) ⇒ no Form 8960,
        //     so line 9b is not a line this return has.
        let mut no_niit = p8_return(dec!(30000), None, None);
        no_niit.int_1099.clear();
        assert_eq!(fires(&no_niit), None);

        // ★★★ (5) LINE 15 BINDS ⇒ the allocation would change the tax by $0, so NOTHING IS SAID
        //     (final whole-branch review, P3-3). Wages $150,000 + $100,000 of investment income:
        //     AGI $250,000 ⇒ line 15 = $50,000, line 12 = $100,000, and line 16 = min(12, 15) takes
        //     the line-15 leg. Allocating the whole $10,000 pool drops line 12 to $90,000 — still
        //     above line 15 — so line 16 does not move and the tax does not change.
        //
        //     This is a COMMON btctax shape, not an edge: wages plus a big crypto gain. The advisory
        //     used to tell this filer "your tax is currently OVERSTATED", which is false, and invite
        //     a sworn §1411 allocation election worth nothing.
        let mut binds = p8_return(dec!(30000), None, None);
        binds.w2s = vec![w2(
            Owner::Taxpayer,
            dec!(150000),
            dec!(150000),
            dec!(150000),
        )];
        binds.int_1099 = vec![crate::tax::return_inputs::Form1099Int {
            payer: "Bank".into(),
            box1_interest: dec!(100000),
            ..Default::default()
        }];
        {
            // The premise, asserted: this fixture must really be in the line-15-binding region, or
            // the row proves nothing. A fixture that merely lost its NIIT would also return `None`.
            let ar = assemble_absolute(&binds, &empty_ledger(), &p, &table, 2024);
            assert!(
                ar.niit.tax > Usd::ZERO,
                "row 5 must still OWE NIIT — otherwise it is row 4 again"
            );
            let l15 = ar.niit.magi - crate::tax::tables::niit_threshold(FilingStatus::Single);
            assert!(
                ar.niit.nii > l15,
                "★ and line 12 ({}) must EXCEED line 15 ({l15}) — that is what makes line 15 the \
                 binding leg of line 16's min",
                ar.niit.nii
            );
            assert!(
                crate::tax::return_1040::nii_line9b_bound(&ar) > Usd::ZERO,
                "…while a real allocation pool still exists, so the OLD predicate would have fired"
            );
        }
        assert_eq!(
            fires(&binds),
            None,
            "★★ nothing is forgone, so nothing is said: line 16 is already the line-15 leg and no \
             9b entry up to the bound changes the return by a cent"
        );

        // The text names the amount, the SAVING, and the direction, and OFFERS the ratio without
        // applying it.
        let msg = Advisory::Form8960Line9bNotClaimed {
            bound: dec!(10000),
            saving: dec!(380),
        }
        .message();
        for phrase in [
            "$10,000",
            "$380",
            "OVERSTATED by up to",
            "any reasonable method",
            "ratio of Form 8960 line 8",
        ] {
            assert!(msg.contains(phrase), "advisory must say {phrase:?}: {msg}");
        }
    }

    /// ★★★ **THE TWO NIIT CHAINS MUST AGREE ONCE 9B EXISTS.** `form_8960` feeds
    /// `AbsoluteReturn::total_tax`; `form_8960_lines` feeds the FILED Schedule 2 line 12. While
    /// Part II was structurally zero the two could not diverge; a deduction threaded into only one
    /// of them files a Form 8960 whose own line 17 contradicts the 1040 that carries it.
    ///
    /// Mutation: drop the `line9b` argument at either call site (`return_1040`'s `form_8960` or
    /// `packet`'s `form_8960_lines`) and this reds by $380.
    #[test]
    fn the_absolute_and_printed_niit_chains_agree_on_the_line9b_deduction() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        let ri = p8_return(dec!(30000), None, Some(dec!(10000)));
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        let f8960 = crate::tax::other_taxes::form_8960_lines(
            FilingStatus::Single,
            ar.taxable_interest,
            ar.ordinary_dividends,
            ar.capital_gain,
            ar.printed_inputs.crypto_lending_interest,
            ar.agi,
            ar.printed_inputs.form_8960_line9b,
        )
        .expect("NIIT is owed");
        assert_eq!(f8960.line8, dec!(60000), "line 8 = the $60,000 of interest");
        assert_eq!(f8960.line12, dec!(50000), "line 12 = line 8 − line 11");
        assert_eq!(
            ar.niit.nii,
            dec!(50000),
            "the absolute chain deducts it too"
        );
        assert_eq!(
            ar.niit.tax, f8960.line17,
            "the tax on the 1040 and the tax on the filed Form 8960 are one number"
        );
        // …and the deduction actually moved it: 3.8% × 60,000 = 2,280 without, 1,900 with.
        let without = p8_return(dec!(30000), None, None);
        let ar_none = assemble_absolute(&without, &empty_ledger(), &p, &table, 2024);
        assert_eq!(ar_none.niit.tax - ar.niit.tax, dec!(380));
    }

    /// ★★ **P4 — the refusal must carry the DEADLINE, because the deadline is the whole reason this
    /// is a gate and not an advisory.** A filer who reads "you need an acknowledgment" and files
    /// anyway has destroyed the cure; a filer who reads "you can still get one, until you file" has
    /// not. §170(f)(8)(C), quoted through i1040sca.
    ///
    /// B1 mutation: drop the "whichever is earlier" deadline sentence from either detail and the
    /// matching assertion reds by name.
    #[test]
    fn both_cwa_refusals_quote_the_still_curable_deadline() {
        let p = ty2024_params();
        let table = synthetic_table(2024);
        for answer in [None, Some(false)] {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: Some(false),
                charitable_cwa_obtained: answer,
                schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs {
                    salt_state_estimated_payments: dec!(10000),
                    mortgage_interest_1098: dec!(20000),
                    ..Default::default()
                }),
                w2s: vec![w2(
                    Owner::Taxpayer,
                    dec!(200000),
                    dec!(168600),
                    dec!(200000),
                )],
                ..Default::default()
            };
            let st = donation_state(dec!(4000));
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            let r = screen_absolute(&ri, &ar, &p, &st, 2024)
                .unwrap_or_else(|| panic!("{answer:?} must refuse"));
            let d = r.detail.to_ascii_lowercase();
            for phrase in [
                "by the date you file your return or the due date (including extensions)",
                "whichever is earlier",
                "§170(f)(8)",
            ] {
                assert!(
                    d.contains(phrase),
                    "the {answer:?} refusal must quote {phrase:?}; got: {}",
                    r.detail
                );
            }
        }
    }

    /// ★★★ **PRE-MERGE I-2 — a REGRESSION the r3 fold itself introduced, and a laundering path.**
    ///
    /// r3's I-3 fix correctly stopped refusing a STANDARD-DEDUCTION year that has a declared donation
    /// restriction: such a return claims no §170 deduction and attaches no Form 8283, so the
    /// restriction moves no figure on it. True — **of that year**.
    ///
    /// But `apply_170b` runs UNCONDITIONALLY even in a standard-deduction year, deliberately, so the
    /// carryover ages (Reg §1.170A-10(a)(2)). So the year still produces a `charitable_carryover_out`
    /// computed at **full fair market value** — the very number the filer has just told us is too
    /// large — and the write-back persisted it into next year's inputs stamped `Computed`, past every
    /// anti-laundering gate. Next year it deducts with NO gate anywhere: `donations_had_restrictions`
    /// is `PerYear` so it is `None` on that row, and the §G-21 screen reads
    /// `year_donation_deduction(state, Y+1)`, which is `$0` because the gift was made in Y.
    ///
    /// ★ The pre-r3 gate DID refuse this year. The fold closed a false-block and opened a laundering
    /// path one clause over — which is exactly why a fix gets reviewed like any other change.
    ///
    /// The invariant, stated once: **do not persist a charitable carryover whose amount btctax cannot
    /// vouch for.** It cannot when a restriction is declared, and it cannot when the restriction
    /// question was DUE (a Section B year) and unanswered.
    ///
    /// Mutation-verified: deleting either arm reds its own row by name.
    #[test]
    fn a_carryover_btctax_cannot_vouch_for_is_never_written_into_next_year() {
        let p = ty2024_params();
        let table = real_2024_table();
        // AGI $30,000 against a $50,000 long-term gift: §170(b)'s 30% ceiling allows $9,000, which
        // LOSES to the $14,600 standard deduction — so nothing is itemized and the year files clean,
        // while $41,000 of full-FMV carryover rolls out.
        // ★★★ FINAL-REVIEW FINDING 1 — `cwa` IS NOW A FIXTURE AXIS, AND IT HAS TO BE.
        //
        // Every household in this test is the standard-deduction deferral donor: a gift over $250
        // that rolls a carryover out of a year that itemizes nothing. That is precisely the
        // population the new §170(f)(8) write-back guard refuses, so rows (3) and (4) — which assert
        // the write-back SUCCEEDS — would now be refused for a reason that has nothing to do with
        // the restriction question this test is about. Answering the acknowledgment `Some(true)`
        // isolates the variable under test, and rows (5)/(6) below assert the new guard is real by
        // flipping only this axis.
        let build_wages = |answer: Option<bool>, claimed: Usd, wages: Usd, cwa: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: answer,
                charitable_cwa_obtained: cwa,
                w2s: vec![w2(Owner::Taxpayer, wages, wages, wages)],
                ..Default::default()
            };
            let st = state_removals(vec![donation(
                date!(2024 - 06 - 01),
                vec![donation_leg(Term::LongTerm, dec!(5000), claimed)],
            )]);
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            (ri, st, ar)
        };
        // The default for these rows holds the acknowledgment, so §170(f)(8) is satisfied and the
        // RESTRICTION question is the only thing that can move the outcome.
        let build = |answer: Option<bool>, claimed: Usd| {
            build_wages(answer, claimed, dec!(30000), Some(true))
        };

        // The fixture must be the shape the finding describes, or the test proves nothing.
        let (ri, st, ar) = build(Some(true), dec!(50000));
        assert!(
            !ar.deduction_is_itemized,
            "fixture must take the STANDARD deduction"
        );
        assert!(
            !ar.charitable_carryover_out.is_empty(),
            "…and must still roll a carryover OUT (apply_170b runs unconditionally)"
        );
        assert_eq!(
            screen_absolute(&ri, &ar, &p, &st, 2024),
            None,
            "…and the YEAR itself is correctly computable — r3's I-3 fix stands"
        );

        // (1) DECLARED restriction ⇒ the carryover is known-inflated. Refuse to persist it.
        //
        // ★ The refusal's REASON is asserted, not merely its existence. There are now three gates in
        //   `apply_carryover_writeback` that can refuse this household, so `is_err()` alone would go
        //   on passing if the restriction arm were deleted outright — the vacuity shape KAT 8 was.
        let e1 = apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false)
            .expect_err(
                "★ a full-FMV carryover from a gift the filer said was restricted must not become \
                 next year's input — next year has no gate that could catch it",
            );
        assert!(
            e1.contains("restriction or a retained right"),
            "…and it must refuse for the RESTRICTION, not for something else. Got: {e1}"
        );

        // (2) DUE-BUT-UNANSWERED on a Section B year ⇒ btctax cannot vouch for the amount either.
        let (ri, st, ar) = build(None, dec!(50000));
        let e2 = apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false)
            .expect_err(
            "the form would have asked 5a/5b/5c; an unanswered Section B year cannot vouch for \
                 the carryover's amount",
        );
        assert!(
            e2.contains("lines 5a, 5b and 5c"),
            "…and for the UNANSWERED Section B question specifically. Got: {e2}"
        );

        // (3) ANSWERED NO ⇒ the ordinary case, and it writes.
        let (ri, st, ar) = build(Some(false), dec!(50000));
        assert!(
            apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false).is_ok(),
            "\"no strings\" is the common case and must not be blocked"
        );

        // (4) SECTION A + unanswered ⇒ the form never poses the question, so silence forgoes nothing.
        //     $4,000 is under the $5,000 split, and $10,000 of wages puts the 30% ceiling at $3,000
        //     so a carryover still rolls out — otherwise this row would prove nothing.
        let (ri, st, ar) = build_wages(None, dec!(4000), dec!(10000), Some(true));
        assert!(
            !ar.charitable_carryover_out.is_empty(),
            "fixture must still carry over"
        );
        assert!(
            apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false).is_ok(),
            "a Section A year never prints 5a/5b/5c — do not block a small donor's carryover"
        );

        // ══ FINAL-REVIEW FINDING 1 — THE §170(f)(8) ARM, ON THE SAME FIXTURES. ═══════════════════
        //
        // Rows (3) and (4) succeed ONLY because `cwa` is `Some(true)`. Flip that one axis and both
        // must refuse — otherwise the acknowledgment answer above is grease rather than a premise,
        // and the standard-deduction deferral donor is laundered into next year exactly as the
        // final review described. Each asserts the §170(f)(8) reason by name, so it cannot be
        // satisfied by the restriction arms above.
        for (label, cwa) in [("unanswered", None), ("answered NO", Some(false))] {
            // (3') row (3)'s household — Section B, restriction answered NO — only the CWA differs.
            let (ri, st, ar) = build_wages(Some(false), dec!(50000), dec!(30000), cwa);
            let e = apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false)
                .expect_err(&format!(
                    "{label}: §170(f)(8)(A) disallows a $250-or-more contribution without a \
                     contemporaneous written acknowledgment, so nothing carries into next year and \
                     nothing may be persisted — row (3) succeeds only because it answers yes"
                ));
            assert!(
                e.contains("contemporaneous written acknowledgment"),
                "{label}: row (3) must refuse for §170(f)(8), not for the restriction. Got: {e}"
            );

            // (4') row (4)'s household — Section A, a $4,000 gift, still over the $250 threshold.
            let (ri, st, ar) = build_wages(None, dec!(4000), dec!(10000), cwa);
            let e = apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false)
                .expect_err(&format!(
                    "{label}: the $250 threshold is §170(f)(8)(A)'s, NOT §170(f)(11)(D)'s $5,000 \
                     appraisal split — a Section A gift of $4,000 still needs an acknowledgment"
                ));
            assert!(
                e.contains("contemporaneous written acknowledgment"),
                "{label}: row (4) must refuse for §170(f)(8). Got: {e}"
            );
        }

        // (5) ★ NO charitable carryover ⇒ nothing inflated to persist, so a declared restriction must
        //     NOT block the write-back — the QBI/REIT carryovers still need writing. Without this the
        //     `!is_empty()` term would be an untested clause, which is how r3's I-3 false block got in.
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            donations_had_restrictions: Some(true),
            w2s: vec![w2(
                Owner::Taxpayer,
                dec!(100000),
                dec!(100000),
                dec!(100000),
            )],
            qbi: QbiInputs {
                reit_ptp_carryforward_in: dec!(10000),
                ..Default::default()
            },
            ..Default::default()
        };
        let st = LedgerState::default(); // no donations at all
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        assert!(
            ar.charitable_carryover_out.is_empty() && ar.qbi_reit_ptp_carryforward_out > Usd::ZERO,
            "fixture: no charitable carryover, but a REIT one that still must be written"
        );
        assert!(
            apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false).is_ok(),
            "a restriction declaration with no charitable carryover blocks nothing"
        );

        // (6) ★ FOLD-REVIEW Minor — a year with NO donation but a rolled-in charitable carryover is
        //     NOT this gate's business. Refusing it printed a message asserting btctax had valued a
        //     donation at full FMV, which is false: there was no donation to value.
        let ri_no_gift = ReturnInputs {
            filing_status: FilingStatus::Single,
            donations_had_restrictions: Some(true),
            charitable_carryover_in: vec![crate::tax::return_inputs::CharitableCarryItem {
                amount: dec!(5000),
                class: crate::tax::return_inputs::CharitableClass::CapGainProp30,
                origin_year: 2023,
                provenance: CarryProvenance::User,
            }],
            w2s: vec![w2(Owner::Taxpayer, dec!(20000), dec!(20000), dec!(20000))],
            ..Default::default()
        };
        let st_no_gift = LedgerState::default(); // no donations THIS year
        let ar_no_gift = assemble_absolute(&ri_no_gift, &st_no_gift, &p, &table, 2024);
        if !ar_no_gift.charitable_carryover_out.is_empty() {
            assert!(
                apply_carryover_writeback(
                    &ar_no_gift,
                    &ri_no_gift,
                    &st_no_gift,
                    2024,
                    ReturnInputs::default(),
                    false
                )
                .is_ok(),
                "this year's restriction answer is about THIS year's gifts; a prior year's carryover \
                 is that year's business"
            );
        }

        // ★ `--force` must NOT open this. It exists to overwrite a USER figure, not to launder one.
        let (ri, st, ar) = build(Some(true), dec!(50000));
        assert!(
            apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), true).is_err(),
            "--force overrides the user-provenance guard, never the vouch-for guard"
        );
    }

    /// ★★★ **K6 — r3 I-4's KILL, RE-ARMED. A computed ZERO must never silence the advisory.**
    ///
    /// r3 I-4 found `apply_carryover_writeback` stamping `capital_loss_carryforward_in_provenance =
    /// Computed` on a value it never assigned. The damage was SILENCE:
    /// `BenefitCarryoversNotStated` defines "unknown" as `zero && User`, so the false stamp made next
    /// year stop telling a filer that btctax has no capital-loss carryover on file — for a filer who
    /// may genuinely have one.
    ///
    /// (B) writes the value now, so the finding cannot be re-run as "the stamp is unfounded because
    /// nothing is written". Its reasoning is unchanged, and this is the case it still governs: year Y
    /// was NEVER ASKED (`{0,0}` / `User`) **and** produced NO carryover-out. There is nothing to
    /// descend from, so no stamp is founded — and the `grounded` predicate is the only thing standing
    /// between that and the old defect.
    ///
    /// ★★★ **THE ADVISORY IS ASSERTED, NOT ONLY THE STAMP.** The stamp is a means; the advisory is
    /// the harm. And NO VALUE ASSERTION CAN CATCH THIS — the stored amount is `{0,0}` whether the
    /// stamp is written or not, which is exactly why r3 I-4 shipped in the first place.
    ///
    /// Mutation that MUST red: drop the `grounded` predicate (stamp unconditionally) ⇒ BOTH halves.
    #[test]
    fn a_computed_zero_never_silences_the_benefit_carryover_advisory() {
        use crate::tax::advisories::{advisories_for, Advisory};

        // Year Y: no carryover in, nothing asked, and no loss of its own to carry out.
        let ri = plain_ri();
        let ar = ar_with_carryovers();
        assert_eq!(
            ri.capital_loss_carryforward_in,
            Carryforward::default(),
            "premise: year Y was never asked"
        );
        assert_eq!(
            ri.capital_loss_carryforward_in_provenance,
            CarryProvenance::User,
            "premise: …and carries the default provenance, which is what 'nobody said' looks like"
        );
        assert_eq!(
            ar.capital_loss_carryforward_out,
            Carryforward::default(),
            "premise: year Y produced NO carryover-out either — with a carryover-out there would be \
             something to descend from and the stamp would be founded"
        );

        let next = apply_carryover_writeback(
            &ar,
            &ri,
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            next.capital_loss_carryforward_in,
            Carryforward::default(),
            "there is no figure to write"
        );
        assert_eq!(
            next.capital_loss_carryforward_in_provenance,
            CarryProvenance::User,
            "★ …so the provenance must stay at its default. `Computed` here would be a claim that \
             btctax derived the zero from a year it computed, and it derived nothing."
        );

        // ★ AND THE HARM ITSELF: next year still tells the filer btctax has no carryover on file.
        let mut y1 = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(60000), dec!(60000), dec!(60000))],
            ..next.clone()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut y1);
        let ar1 = assemble_absolute(
            &y1,
            &empty_ledger(),
            &ty2024_params(),
            &real_2024_table(),
            2024,
        );
        let advs = advisories_for(&y1, &empty_ledger(), &ar1, &ty2024_params(), 2024);
        assert!(
            advs.iter().any(|a| matches!(
                a,
                Advisory::BenefitCarryoversNotStated {
                    capital_loss: true,
                    ..
                }
            )),
            "★★★ THE r3 I-4 DAMAGE, asserted directly: next year must still say it has no \
             capital-loss carryover on file. A false `Computed` stamp silences exactly this, and \
             every value assertion above stays green while it does. Got: {advs:?}"
        );

        // The CHARITABLE sibling is honest — its value IS assigned one line above its stamp — so the
        // `grounded` gate must not have been generalised into removing that.
        assert_eq!(
            next.charitable_carryover_in_provenance,
            CarryProvenance::Computed,
            "the charitable stamp is founded: `charitable_carryover_out` is real and is assigned"
        );
    }

    /// ★★★ **K7 — the stamp is founded ONLY because the value is assigned.**
    ///
    /// The positive half of r3 I-4. Year Y brings a real `User` carryover in and produces a real
    /// carryover-out, so the figure descends from the filer's own testimony: btctax may say
    /// `Computed`, because it computed it.
    ///
    /// Mutation that MUST red: remove the value assignment and keep the stamp ⇒ the VALUE assertion
    /// reds. The test must be unable to pass on a stamp alone, which is the failure mode of the
    /// version this replaces.
    #[test]
    fn the_writeback_stamps_computed_only_because_it_assigns_the_value() {
        let p = ty2024_params();
        let table = real_2024_table();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(60000), dec!(60000), dec!(60000))],
            donations_had_restrictions: Some(false),
            ..Default::default()
        };
        ri.capital_loss_carryforward_in = Carryforward {
            short: Usd::ZERO,
            long: dec!(5000),
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        assert!(
            ar.capital_loss_carryforward_out.long > Usd::ZERO,
            "premise: there must BE a carryover-out to roll, or this asserts nothing"
        );

        let next = apply_carryover_writeback(
            &ar,
            &ri,
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            next.capital_loss_carryforward_in,
            Carryforward {
                short: round_dollar(ar.capital_loss_carryforward_out.short),
                long: round_dollar(ar.capital_loss_carryforward_out.long),
            },
            "★ the VALUE is written — and rounded to whole dollars, because it becomes next year's \
             Schedule D lines 6 and 14, which the filer reads off the page and swears to"
        );
        assert_eq!(
            next.capital_loss_carryforward_in_provenance,
            CarryProvenance::Computed,
            "…and only now is the stamp founded"
        );
    }

    /// ★★★ **K16 — the write-back is ATOMIC, and the reason is a TAX mechanism, not tidiness.**
    ///
    /// It is tempting to let the capital-loss half survive a charitable refusal: the two carryovers
    /// look independent. They are not. Worksheet line 1 is `agi − total_deductions`, so a charitable
    /// deduction btctax cannot vouch for makes line 1 **more negative**, line 3 and line 4 **smaller**,
    /// and the surviving capital loss **LARGER**. Persisting that half alone would overstate next
    /// year's carryover using a deduction btctax has just refused to stand behind.
    ///
    /// ★★★ **THE ATOMICITY ITSELF IS STRUCTURAL, NOT TESTED — and saying so is the point.**
    /// `apply_carryover_writeback` takes `next_year` **by value** and returns
    /// `Result<ReturnInputs, String>`, so on the `Err` path the mutated copy is dropped and the caller
    /// keeps its own. A partial application is not merely absent; it is unrepresentable, which is a
    /// stronger guarantee than any assertion and the reason none is written for it. A test asserting
    /// "nothing was applied" against a value the caller never receives would be theatre.
    ///
    /// So what IS asserted is what a filer can actually be harmed by:
    ///   (a) the refusal TEXT names the capital-loss carryover among what was withheld. It enumerates
    ///       what was not persisted so the filer can enter it by hand, and that enumeration has gone
    ///       short once already (it said "all three" while the code wrote three and is now four);
    ///   (b) the same household WITH a charitable deduction has a strictly LARGER capital-loss
    ///       carryover-out than without it, which is the mechanism spelled out above, executed. This
    ///       is what makes (a) a correctness rule and not housekeeping.
    ///
    /// Mutations that MUST red:
    ///   (a) drop "CAPITAL-LOSS" from the §170(f)(8) refusal text ⇒ the enumeration half;
    ///   (b) pass the FLOORED `taxable_income` as worksheet line 1 instead of the signed one ⇒ the
    ///       second half, because at the floor the two deductions stop being distinguishable.
    #[test]
    fn an_unvouched_charitable_deduction_blocks_the_capital_loss_roll_too() {
        let p = ty2024_params();
        let table = synthetic_table(2024);

        // An itemizing floor-ish household with a real capital loss of its own AND a $4,000 gift
        // whose §170(f)(8) acknowledgment has not been declared.
        // ★ The $18,000 of mortgage interest is not decoration: it is what makes BOTH households
        //   itemize, so the charitable deduction actually reaches `total_deductions` and therefore
        //   worksheet line 1. Without it the ceiling-limited gift loses to the standard deduction and
        //   the second half compares two identical returns.
        let sched_a = || {
            Some(crate::tax::return_inputs::ScheduleAInputs {
                mortgage_interest_1098: dec!(18000),
                ..Default::default()
            })
        };
        let build = |cwa: Option<bool>, gift: bool, itemize: bool| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                donations_had_restrictions: Some(false),
                charitable_cwa_obtained: cwa,
                schedule_a: if itemize { sched_a() } else { None },
                w2s: vec![w2(Owner::Taxpayer, dec!(20000), dec!(20000), dec!(20000))],
                ..Default::default()
            };
            ri.capital_loss_carryforward_in = Carryforward {
                short: Usd::ZERO,
                long: dec!(60000),
            };
            crate::tax::testonly::answer_all_live_declarations(&mut ri);
            let st = if gift {
                donation_state(dec!(20000))
            } else {
                LedgerState::default()
            };
            let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
            (ri, st, ar)
        };

        // ── (a) the refusal blocks everything, and SAYS SO. ──────────────────────────────────────
        // ★ STANDARD DEDUCTION here, deliberately — this refusal only ever bites the
        //   standard-deduction DEFERRAL donor in production: an itemizer with an unanswered
        //   acknowledgment refuses at `screen_absolute` and never reaches the write-back at all. A
        //   fixture that itemized would be exercising a state the product cannot get into.
        let (ri, st, ar) = build(None, true, false);
        assert!(
            cwa_unvouched_carryover(&ri, &ar, &st, 2024).is_some(),
            "premise: the §170(f)(8) gate must actually be armed on this fixture, or the atomicity \
             claim below is never exercised"
        );
        assert!(
            ar.capital_loss_carryforward_out.long > Usd::ZERO,
            "premise: there must BE a capital-loss carryover to leak, or 'nothing was applied' is \
             vacuously true"
        );
        let err = apply_carryover_writeback(&ar, &ri, &st, 2024, ReturnInputs::default(), false)
            .expect_err("an unvouched §170(f)(8) carryover refuses the whole write-back");
        assert!(
            err.contains("CAPITAL-LOSS"),
            "★ the refusal must TELL the filer the capital-loss carryover was not written either — \
             the text enumerates what was withheld, and it went short once already. Got: {err}"
        );

        // ── (b) the MECHANISM: the charitable deduction makes the carryover-out LARGER. ────────────
        //
        // Same household, acknowledgment declared, so the write succeeds — and compare against the
        // same household with no gift at all.
        // ★ ITEMIZING here, equally deliberately: the mechanism under test is the charitable
        //   deduction moving worksheet line 1, and a deduction that loses to the standard deduction
        //   moves nothing. The acknowledgment is declared, so both returns are ones btctax will file.
        let (_, _, ar_gift) = build(Some(true), true, true);
        let (_, _, ar_no) = build(Some(true), false, true);

        let w_gift = ar_gift
            .capital_loss_carryover_worksheet
            .expect("the gift household uses the worksheet");
        let w_no = ar_no
            .capital_loss_carryover_worksheet
            .expect("the no-gift household uses the worksheet");
        assert!(
            w_gift.line1 < w_no.line1,
            "★ the deduction makes worksheet line 1 MORE negative: {} vs {}",
            w_gift.line1,
            w_no.line1
        );
        assert!(
            w_gift.line4 < w_no.line4,
            "…so line 4 (what the year actually absorbed) is SMALLER: {} vs {}",
            w_gift.line4,
            w_no.line4
        );
        assert!(
            ar_gift.capital_loss_carryforward_out.long > ar_no.capital_loss_carryforward_out.long,
            "★★ …and the surviving loss is therefore strictly LARGER, which is exactly why the \
             capital-loss half must not outlive a charitable refusal: {} vs {}",
            ar_gift.capital_loss_carryforward_out.long,
            ar_no.capital_loss_carryforward_out.long
        );
    }

    /// ★★★ **K8 — a USER-ENTERED capital-loss carryover is never overwritten without `--force`.**
    ///
    /// The fourth `!force` guard. Next year's lines 6 and 14 are the filer's own testimony; btctax's
    /// figure must not silently replace it.
    ///
    /// ★ **A test exercising only `force = true` would pass with the guard deleted and be worthless**,
    /// so the `force = false` half is the one that carries the weight — and the refusal TEXT is
    /// asserted, because several refusals can come out of this function and "it errored" is not
    /// evidence that THIS one fired.
    ///
    /// ★★ The third case is the one a naive guard breaks: a FRESH `{0,0}` next-year row also carries
    /// `User` (it is the `CarryProvenance` default), so a guard keyed on provenance alone would refuse
    /// every first write. A zero nobody entered is not testimony.
    ///
    /// Mutation that MUST red: delete the fourth arm ⇒ the `force = false` half.
    #[test]
    fn a_user_entered_capital_loss_carryover_is_not_overwritten_without_force() {
        let p = ty2024_params();
        let table = real_2024_table();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(60000), dec!(60000), dec!(60000))],
            donations_had_restrictions: Some(false),
            ..Default::default()
        };
        ri.capital_loss_carryforward_in = Carryforward {
            short: Usd::ZERO,
            long: dec!(5000),
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        let ar = assemble_absolute(&ri, &empty_ledger(), &p, &table, 2024);
        let computed = Carryforward {
            short: round_dollar(ar.capital_loss_carryforward_out.short),
            long: round_dollar(ar.capital_loss_carryforward_out.long),
        };
        assert_ne!(
            computed,
            Carryforward {
                short: Usd::ZERO,
                long: dec!(40000)
            },
            "premise: btctax's figure must DIFFER from the user's, or 'not overwritten' is unfalsifiable"
        );

        // The filer typed their own carryover onto next year's row.
        let user_row = ReturnInputs {
            capital_loss_carryforward_in: Carryforward {
                short: Usd::ZERO,
                long: dec!(40000),
            },
            capital_loss_carryforward_in_provenance: CarryProvenance::User,
            ..Default::default()
        };

        let err =
            apply_carryover_writeback(&ar, &ri, &empty_ledger(), 2024, user_row.clone(), false)
                .expect_err("★ a user-entered carryover must not be silently overwritten");
        assert!(
            err.contains("capital-loss carryover was user-entered") && err.contains("--force"),
            "the refusal must name THIS carryover and the way out — several refusals leave this \
             function, so a bare `is_err()` proves nothing. Got: {err}"
        );

        // …and `--force` is the way out.
        let forced =
            apply_carryover_writeback(&ar, &ri, &empty_ledger(), 2024, user_row, true).unwrap();
        assert_eq!(
            forced.capital_loss_carryforward_in, computed,
            "`--force` overwrites it with the computed §1212(b) figure"
        );

        // ★ A FRESH `{0,0}` / `User` row still writes without `--force`.
        let fresh = apply_carryover_writeback(
            &ar,
            &ri,
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .expect("a fresh next-year row is not the filer's testimony and must not be wedged");
        assert_eq!(fresh.capital_loss_carryforward_in, computed);
    }

    /// Write-back into a FRESH next year: the computed carryovers become next year's carryover-in, stamped
    /// `Computed` (so a subsequent report can overwrite them silently).
    #[test]
    fn writeback_into_fresh_next_year() {
        let ar = ar_with_carryovers();
        let next = apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            ReturnInputs::default(),
            false,
        )
        .unwrap();
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
                qbi_carryforward_in: Usd::ZERO,
                qbi_carryforward_in_provenance: CarryProvenance::User,
                reit_ptp_carryforward_in: dec!(999),
                reit_ptp_carryforward_in_provenance: CarryProvenance::Computed,
            },
            ..Default::default()
        };
        let next = apply_carryover_writeback(&ar, &plain_ri(), &empty_ledger(), 2024, prior, false)
            .unwrap();
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
        assert!(apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            user_charitable.clone(),
            false
        )
        .is_err());
        assert!(apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            user_charitable,
            true
        )
        .is_ok()); // --force overwrites
                   // User QBI carryforward present → refuse without force (atomic: charitable not half-written).
        let user_qbi = ReturnInputs {
            qbi: QbiInputs {
                qbi_carryforward_in: Usd::ZERO,
                qbi_carryforward_in_provenance: CarryProvenance::User,
                reit_ptp_carryforward_in: dec!(3000),
                reit_ptp_carryforward_in_provenance: CarryProvenance::User,
            },
            ..Default::default()
        };
        assert!(apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            user_qbi.clone(),
            false
        )
        .is_err());
        assert!(
            apply_carryover_writeback(&ar, &plain_ri(), &empty_ledger(), 2024, user_qbi, true)
                .is_ok()
        );
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
        assert!(apply_carryover_writeback(
            &ar,
            &plain_ri(),
            &empty_ledger(),
            2024,
            ri.clone(),
            false
        )
        .is_err());
        assert!(
            apply_carryover_writeback(&ar, &plain_ri(), &empty_ledger(), 2024, ri, true).is_ok()
        );
    }

    // ── N1 — the §1212(b)(2)(B) Capital Loss Carryover Worksheet on the carryforward-OUT side ─────

    /// **L4** — the plan's floor vector: Single, NO wages, one long-term crypto loss of $20,000
    /// (bought $60,000, sold $40,000). AGI = −3,000; standard deduction $14,600 ⇒ 1040 line 15 would
    /// be **−17,600** if the form let you write it, so the §1211(b) $3,000 was never actually
    /// absorbed and the whole $20,000 survives into 2025.
    fn l4_floor_loss_year() -> (ReturnInputs, LedgerState) {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: crate::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        let st = state_disposals(vec![disp_leg(Term::LongTerm, dec!(40000), dec!(60000))]);
        (ri, st)
    }

    /// **L3** — the same $20,000 long-term loss against $40,000 of wages. Taxable income is POSITIVE
    /// ($22,400), the $3,000 IS absorbed, and the flat rule's $17,000 is the worksheet's answer too.
    fn l3_positive_ti_loss_year() -> (ReturnInputs, LedgerState) {
        let mut ri = wages_single(dec!(40000));
        ri.header = crate::tax::testonly::not_a_dependent();
        let st = state_disposals(vec![disp_leg(Term::LongTerm, dec!(40000), dec!(60000))]);
        (ri, st)
    }

    /// The frozen crypto-delta engine's carryforward-out for the same household — the figure
    /// `report --tax-year` prints today, and the one that was measured at 17,000 on L4.
    fn delta_engine_carryforward(ri: &ReturnInputs, st: &LedgerState) -> Carryforward {
        let prof = derive_tax_profile(ri, &ty2024_params(), 2024);
        match compute_tax_year(&[], st, 2024, Some(&prof), &tables_2024()) {
            TaxOutcome::Computed(r) => r.carryforward_out,
            TaxOutcome::NotComputable(b) => panic!("the fixture must compute: {b:?}"),
        }
    }

    /// ★★★ **N1, the defect vector.** A loss year whose taxable income is at the floor carries the
    /// WHOLE $20,000 forward, not $17,000 — the §1211(b) allowance was never absorbed, because there
    /// was no income for it to offset.
    ///
    /// The old flat rule (`carry = loss − min(loss, $3,000)`) is not merely imprecise here: it hands
    /// the filer a permanently smaller loss, and next year's M4 consistency check then *disputes the
    /// correct figure* if they work the worksheet themselves. This assertion reds against the
    /// pre-N1 tree at `left: 17000`.
    #[test]
    fn n1_a_loss_year_at_the_floor_carries_the_whole_loss() {
        let (ri, st) = l4_floor_loss_year();
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);

        // The fixture must actually BE the floor case, or the test proves nothing.
        assert_eq!(
            ar.agi,
            dec!(-3000),
            "wages 0 + L7 −3,000 (§1211(b)-limited)"
        );
        assert_eq!(ar.taxable_income, Usd::ZERO, "1040 L15 is floored at zero…");
        assert_eq!(
            ar.agi - ar.total_deductions,
            dec!(-17600),
            "…and the worksheet's line 1 is the −17,600 the floor hides"
        );
        assert_eq!(ar.schedule_d.loss_deduction_21, dec!(3000));

        let w = ar
            .capital_loss_carryover_worksheet
            .expect("line 21 is a loss and line 1 is below zero");
        assert_eq!(w.line1, dec!(-17600));
        assert_eq!(w.line3, Usd::ZERO, "combine 1 and 2 ⇒ below zero ⇒ -0-");
        assert_eq!(w.line4, Usd::ZERO, "NOTHING of the $3,000 was absorbed");
        assert_eq!(w.line13, Some(dec!(20000)));

        assert_eq!(
            ar.capital_loss_carryforward_out,
            Carryforward {
                short: Usd::ZERO,
                long: dec!(20000),
            },
            "the §1212(b)(2)(B) worksheet carries the whole loss into 2025"
        );

        // ★ And the split that makes this worth stating out loud: the FROZEN delta engine still says
        //   17,000, and structurally cannot say otherwise — `TaxProfile` hands it an
        //   `ordinary_taxable_income` already floored at zero, so the worksheet's line 1 does not
        //   survive into it. That is why the fix lives on the full return and why the advisory below
        //   exists to reconcile the two numbers for the filer.
        assert_eq!(
            delta_engine_carryforward(&ri, &st).long,
            dec!(17000),
            "the frozen slice engine is unchanged — if this moves, `compute.rs` was edited"
        );
    }

    /// ★★★ **N1's no-op half.** At POSITIVE taxable income the flat rule is exact, and the worksheet
    /// must agree with it to the cent — on both characters, and with the frozen engine.
    ///
    /// Without this, a fix that simply carried the whole loss whenever a loss existed would pass the
    /// vector above and overstate every ordinary loss year by $3,000.
    #[test]
    fn n1_at_positive_taxable_income_the_worksheet_is_a_no_op() {
        let (ri, st) = l3_positive_ti_loss_year();
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);

        assert_eq!(ar.taxable_income, dec!(22400), "37,000 AGI − 14,600 std");
        let w = ar.capital_loss_carryover_worksheet.expect("limb (a)");
        assert_eq!(w.line4, dec!(3000), "the full allowance WAS absorbed");
        assert_eq!(
            ar.capital_loss_carryforward_out,
            Carryforward {
                short: Usd::ZERO,
                long: dec!(17000),
            }
        );
        assert_eq!(
            ar.capital_loss_carryforward_out,
            delta_engine_carryforward(&ri, &st),
            "at positive TI the worksheet and the frozen flat rule are the SAME number"
        );
    }

    /// A gain year has no worksheet and no carryforward — the applicability sentence's "Otherwise,
    /// you don't have any carryovers", reached through the real assembly rather than in isolation.
    #[test]
    fn n1_a_gain_year_produces_no_worksheet_and_no_carryforward() {
        let ri = wages_single(dec!(60000));
        let st = state_disposals(vec![disp_leg(Term::LongTerm, dec!(50000), dec!(20000))]);
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        assert_eq!(ar.schedule_d.loss_deduction_21, Usd::ZERO);
        assert_eq!(ar.capital_loss_carryover_worksheet, None);
        assert_eq!(ar.capital_loss_carryforward_out, Carryforward::default());
    }

    /// ★★★ **N1's reader.** A computed figure nobody reads is not thereby correct, and the number the
    /// `report` command prints comes from the FROZEN delta engine, which structurally cannot apply the
    /// worksheet. So the worksheet's answer is delivered as an advisory that names BOTH numbers and
    /// says which to carry — otherwise the fix would be invisible to the only person it is for.
    ///
    /// **B1 pair**: it fires on the floor vector, and it must NOT fire on the positive-TI vector,
    /// which is the same assertion as "the worksheet is a no-op there" made through a second surface.
    #[test]
    fn n1_the_worksheet_result_reaches_the_filer_as_an_advisory() {
        use crate::tax::advisories::{advisories_for, Advisory};
        let p = ty2024_params();
        let table = real_2024_table();

        let (ri, st) = l4_floor_loss_year();
        let ar = assemble_absolute(&ri, &st, &p, &table, 2024);
        let advs = advisories_for(&ri, &st, &ar, &p, 2024);
        let fired = advs
            .iter()
            .find(|a| {
                matches!(
                    a,
                    Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover { .. }
                )
            })
            .expect("the floor vector must be told its carryover is $20,000, not $17,000");
        assert_eq!(
            *fired,
            Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover {
                flat_short: Usd::ZERO,
                flat_long: dec!(17000),
                worksheet_short: Usd::ZERO,
                worksheet_long: dec!(20000),
                absorbed: Usd::ZERO,
            }
        );
        let msg = fired.message();
        assert!(msg.contains("$20,000"), "the CORRECT figure: {msg}");
        assert!(
            msg.contains("$17,000"),
            "…and the one already printed: {msg}"
        );
        assert!(msg.contains("$3,000"), "…and what is at stake: {msg}");

        // The negative half: an ordinary loss year, where the flat rule is exact, must stay silent.
        let (ri3, st3) = l3_positive_ti_loss_year();
        let ar3 = assemble_absolute(&ri3, &st3, &p, &table, 2024);
        assert!(
            !advisories_for(&ri3, &st3, &ar3, &p, 2024).iter().any(|a| {
                matches!(
                    a,
                    Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover { .. }
                )
            }),
            "at positive taxable income there is nothing to reconcile — an advisory here would be \
             noise on every loss year btctax already gets right"
        );
    }

    /// ★ The N1 fix must not have moved the FILED return. Every printed figure on the floor vector —
    /// 1040 line 7, taxable income, total tax — is what it was before the worksheet existed; the
    /// worksheet decides only what survives into next year.
    #[test]
    fn n1_does_not_move_a_single_filed_figure() {
        let (ri, st) = l4_floor_loss_year();
        let ar = assemble_absolute(&ri, &st, &ty2024_params(), &real_2024_table(), 2024);
        assert_eq!(ar.capital_gain, dec!(-3000), "1040 L7, leading minus");
        assert_eq!(ar.total_income, dec!(-3000)); // L9
        assert_eq!(ar.taxable_income, Usd::ZERO); // L15
        assert_eq!(ar.regular_tax, Usd::ZERO); // L16
        assert_eq!(ar.total_tax, Usd::ZERO); // L24
        assert_eq!(ar.schedule_d.lt_net_15, dec!(-20000));
        assert_eq!(ar.schedule_d.total_16, dec!(-20000));
    }
}
