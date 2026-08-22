//! Full-return v1 **advisories** (Phase 5 / SPEC §3.4 + §9.2) — the loud, non-gating notes the report must
//! surface alongside a computed return.
//!
//! Two kinds, and the distinction is the whole point of §3.4:
//! - **Conservative omissions.** A purely taxpayer-*favorable* benefit v1 deliberately does not compute
//!   (CTC/ODC, EIC) is NOT refused — omitting it can only OVERSTATE tax, never understate it. But the filer
//!   must be *told*, or the overstatement is silent. Same for a §63(f) aged box forfeited for want of a DOB.
//! - **Disclosures.** Facts the filer must decide for themselves (FBAR/FinCEN; the charitable-donee class
//!   the ledger assumed). v1 never auto-answers these.
//!
//! Every advisory here is **non-gating**: it never changes a number and never changes the exit code. The
//! things that *would* make the return wrong are refusals (`return_refuse.rs`), not advisories.
use crate::conventions::Usd;
use crate::state::{LedgerState, RemovalKind};
use crate::tax::return_1040::{mixed_use_mortgage_forgone, AbsoluteReturn};
use crate::tax::return_inputs::ReturnInputs;
use crate::tax::tables::FullReturnParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// The AGI ceiling below which the **EIC** advisory fires. Deliberately a round over-estimate of the
/// TY2024 maximum EIC AGI, which is **$66,819** — MFJ with 3+ qualifying children (Rev. Proc.
/// 2023-34 §2.06). This advisory only decides whether to SHOW a "you may qualify" note, never a
/// computed figure, so over-firing is the safe direction and UNDER-firing is the bug.
///
/// **[★ P5-I3]** This was $60,000, from a comment that misread the table: $59,899 is the
/// *Single/HoH/QSS* 3-child limit, not the MFJ one. Every MFJ band above it was therefore missed —
/// an MFJ household with 3 children and $63,000 of AGI (max credit $7,830) got no advisory at all,
/// which is precisely the direction §3.4's conservative-omission carve-out promises never to fail in.
/// $70,000 is a round number safely above the real ceiling, with headroom for several years of
/// inflation adjustment. The full TY2024 AGI limits, for the record:
///
/// | qualifying children | Single / HoH / QSS | MFJ     |
/// |---------------------|--------------------|---------|
/// | 0                   | $18,591            | $25,511 |
/// | 1                   | $49,084            | $56,004 |
/// | 2                   | $55,768            | $62,688 |
/// | 3+                  | $59,899            | **$66,819** |
const EIC_ADVISORY_AGI_CEILING: Usd = dec!(70000);

/// A non-gating advisory on a computed full return (SPEC §3.4 / §9.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advisory {
    /// §3.4 conservative omission: the Child Tax Credit / Credit for Other Dependents is not computed
    /// (1040 L19 is pinned to $0) even though dependents were captured. Overstates tax.
    ///
    /// ★★★ `provably_zero` is set when **Schedule 8812's own arithmetic** kills the credit whatever the
    /// dependents' ages are. The advisory used to fire unconditionally and tell every filer with a
    /// dependent that their tax was *"OVERSTATED by up to $2,000 per qualifying child"* — a filing
    /// trial caught it telling a filer with AGI $2,085,000 and nine children to claim $18,000 that
    /// §24(b) had already reduced by $84,250. Line 19 was $0 and correct; the ADVICE was not, and it
    /// sent the filer to Schedule 8812 for nothing.
    CtcOdcOmitted {
        dependents: usize,
        provably_zero: bool,
    },
    /// §3.4 conservative omission: the Earned Income Credit is not computed and the household's income is
    /// low enough that it might qualify. Overstates tax.
    EicOmitted,
    /// §63(f): a person's date of birth is not on file, so the aged (65+) additional standard deduction is
    /// NOT granted (never granted on an unsubstantiated birthdate). Overstates tax if they are 65+.
    AgedBoxForfeitedNoDob { per_box: Usd },
    /// ★ §63(f) / §G-9: a person's date of birth IS on file and DOES qualify them for the aged box, but
    /// the "died during the year?" question was skipped — so `is_aged` forgoes the addition rather than
    /// grant it on an unresolved death carve-out. Overstates tax, and unlike
    /// [`Self::AgedBoxForfeitedNoDob`] the filer has given us everything except one yes/no, so this is
    /// the cheapest advisory on the list to act on. `persons` counts taxpayer + (on MFJ) spouse.
    AgedBoxForfeitedDeathUnanswered { per_box: Usd, persons: usize },
    /// FinCEN Notice 2020-2 disclosure — the filer declared a foreign financial account. v1 never
    /// auto-answers Schedule B Part III.
    FbarFinCen,
    /// ★ Schedule B line 7a is **Yes** but its unnumbered FBAR sub-question was SKIPPED, so the box
    /// prints blank. Lawful — no figure reads it, and a blank is no testimony — but the form carries a
    /// Caution about substantial penalties, so skipping it may not be silent. Quotes the Caution
    /// verbatim. Fires ONLY on the skip (`None`); an answered box, either way, needs no advisory.
    FbarSubQuestionNotAnswered,
    /// ★ Schedule C line **I** (and, when reached, line **J**) — the Form-1099 compliance pair — was
    /// SKIPPED, so the box(es) print blank. Lawful: no figure on the return reads them, and unlike
    /// Schedule B's FBAR sub-question the form prints no Caution beside them. But the §6721/§6722
    /// exposure is real and independent of the box, so the skip is said out loud. `line_j_too` is
    /// `true` when line I was answered **Yes** and line J was then skipped — a strictly worse place to
    /// stop, since the filer has already declared the payments exist.
    ScheduleC1099NotAnswered { line_j_too: bool },
    /// ★★ §G-22 — the return claims a §199A deduction, and btctax holds **no** prior-year QBI loss
    /// carryforward AND did not compute the prior year itself (`CarryProvenance::User` on a zero).
    ///
    /// Both carryforwards (Form 8995 lines **7** and **3**) are prior-year LOSSES that REDUCE the
    /// deduction, so a zero btctax merely *assumed* inflates it and **UNDERSTATES the tax** — the one
    /// direction §3.4 never permits silently. Unlike the capital-loss and charitable carryovers, whose
    /// absence only forgoes a benefit, these two cost the Treasury rather than the filer.
    ///
    /// ★ **Deliberately fires for most first-time users, and that is correct.** btctax cannot
    /// distinguish "no carryforward" from "a carryforward it was never told about" — a zero it wrote
    /// itself after computing last year is knowledge; a zero that is merely the struct default is not.
    /// `CarryProvenance` is exactly that distinction, so the advisory goes quiet the moment btctax has
    /// computed a prior year, or the filer states a figure of their own.
    QbiCarryforwardNotStated,
    /// ★★ §G-20 — an **MFS** return whose spouse would qualify for a §63(f) aged and/or blind box, and
    /// the box is forgone because one of i1040gi's three conditions is UNANSWERED.
    ///
    /// i1040gi permits them on MFS *"if your spouse had no income, isn't filing a return, and can't be
    /// claimed as a dependent on another person's return"*. btctax captures all three and CLAIMS the
    /// boxes when all three are affirmatively met (`questions::spouse_63f_status_permits`); the gate
    /// fails closed, so silence forgoes. That is lawful (it OVERSTATES tax) **but §3.4 permits it only
    /// if the filer is TOLD** — hence this advisory.
    ///
    /// ★★★ PRE-MERGE finding 1 — IT FIRES ONLY WHEN ANSWERING COULD STILL RECOVER THE BOX. If the
    /// filer answered a condition ADVERSELY (the spouse had income, or files their own return), the
    /// boxes are correctly declined, nothing is recoverable, and advising a hand-claim would invite an
    /// UNDERSTATEMENT on a §6065 return. The doc and message here previously asserted the pre-`fd9c15f`
    /// world — "counts spouse boxes on MFJ only", "three conditions btctax captures none of" — for a
    /// full branch after both became false. `boxes` counts what was forgone: aged, blind, or both.
    Mfs63fSpouseBoxesForgone { per_box: Usd, boxes: usize },
    /// ★★★ §6413(c) — one employer over-withheld Social Security, so the excess is **not creditable on
    /// this return**, and the filer must be TOLD where their money is.
    ///
    /// btctax used to REFUSE here. i1040gi says *"you can't claim the excess on your return"*, not "you
    /// can't file", so the return now files with a $0 credit — but a conservative omission is permitted
    /// **only if the filer is told** (the rule [`Self::BenefitCarryoversNotStated`] states). Without
    /// this, a known, computed, recoverable amount was silently dropped: the review that caught it
    /// noted the code comment already CLAIMED an advisory that had never been written.
    ///
    /// Carries the remedy the instruction gives and that the earlier transcription stopped short of:
    /// *"The employer should adjust the tax for you. If the employer doesn't adjust the overcollection,
    /// you can file a claim for refund using Form 843."*
    ExcessSsNotCreditable {
        /// "you" or "your spouse" — the §3101(a) cap is per person and so is the remedy.
        whose: &'static str,
        /// The employer to ask, canonicalized to nine digits.
        ein: String,
        amount: Usd,
    },
    /// ★★ §G-20a — a prior-year **benefit** carryover btctax was never told about: the §1212(b)
    /// capital-loss carryover and/or the §170(d)(1) charitable carryover.
    ///
    /// **Opposite direction from [`Self::QbiCarryforwardNotStated`], and the message must say so.**
    /// These two REDUCE the filer's tax, so omitting one costs THEM, not the Treasury — a conservative
    /// omission §3.4 permits, **but only if they are told**. The QBI pair goes the other way.
    ///
    /// ★ Gated on `CarryProvenance::User` on an empty/zero value, exactly like the QBI advisory: a zero
    /// btctax computed from a prior year it saw is knowledge; a zero that is the struct default is not.
    BenefitCarryoversNotStated {
        capital_loss: bool,
        charitable: bool,
    },
    /// The ledger classified crypto donations assuming a **public charity (50%-org)** donee. A private
    /// foundation is the 20%-ceiling / basis class (which v1 refuses), so the donee must be verified.
    CharitableDoneeAssumedPublicCharity { donations: usize },
    /// ★★★ **§170(f)(11)(D) — the qualified appraisal must be ATTACHED TO THE RETURN.** Distinct from
    /// §170(f)(11)(C)'s $5,000 rule, which requires the filer to *obtain* an appraisal and keep it:
    /// over $500,000 the appraisal itself is a required attachment, and a return that omits it fails
    /// the substantiation requirement for the whole claim.
    ///
    /// ★★ `claimed` is the **pre-ceiling** amount claimed for the property — post-§170(e) reduction,
    /// aggregated across all similar items given in the year, and determined WITHOUT regard to the
    /// §170(b) AGI ceiling or the §170(d) carryover split. NOT Schedule A line 12: keying it to the
    /// post-ceiling line would make a statutory attachment depend on AGI, and Reg §1.170A-16(f)(3)
    /// extends the same duty to *"the return for any carryover year"*, which only coheres if the
    /// trigger follows the CLAIM across years rather than the annual allowed slice.
    ///
    /// ★ btctax cannot produce the appraisal — only the appraiser can — so this is an advisory with a
    /// matching MANIFEST line, not a refusal: the packet tells the filer exactly what to staple to it.
    QualifiedAppraisalMustBeAttached { claimed: Usd },
    /// §3.4 conservative omission (SPEC §1.2): the education, dependent-care, retirement-savings
    /// (saver's), residential-energy and adoption credits are not computed — Schedule 3 Part I is
    /// $0 apart from the foreign tax credit. Purely taxpayer-FAVORABLE, so it advises, never refuses.
    /// Unconditional on a computed full return: v1 captures no input that would let it decide
    /// eligibility, so it cannot know whether the filer qualifies — only that it did not try.
    OtherCreditsOmitted,
    /// §3.4 / SPEC §9.2 conservative omission: v1 never fills the 1040 direct-deposit block (L35b–d),
    /// so a refund arrives as a **paper check**. Fires only when the return is actually due a refund.
    RefundByPaperCheck { refund: Usd },
    /// ★ §163(h)(3)(F) (P9 §2.7 / §3.4): the filer declared a MIXED-USE mortgage, and v1 cannot compute the
    /// Pub. 936 allocation — so Schedule A line 8a was treated as $0 and the line-8 box was checked. This can
    /// be a LARGE overstatement of tax (a $500k acquisition mortgage with a $20k HELOC forfeits ~96% of a
    /// real deduction). MANDATORY: it names the whole forgone amount as a CEILING ("up to"). Fires on
    /// `Some(false)` — the filer ANSWERED, and answered the way that costs them money. `itemized` records
    /// which deduction the return actually took, so the text does not describe a form the filer did not file
    /// (r5 M-1).
    MixedUseMortgageNotAllocated {
        forgone_interest: Usd,
        itemized: bool,
    },
    /// ★ §63(f) (P9 §2.2 / §3.4): a person's blindness was never declared, so the additional standard
    /// deduction for blindness was NOT granted. Fires on `blind.is_none()` (never asked) — never on
    /// `Some(false)`. `persons` counts the taxpayer's box plus, WHEN A SPOUSE BOX COULD COUNT
    /// (`questions::spouse_63f_status_permits` — MFJ, or a qualifying MFS), the spouse's (an absent
    /// spouse forfeits too; MFS never counts the spouse). Same statute, dollars and worksheet line as the
    /// aged box (`AgedBoxForfeitedNoDob`), and the two STACK. Overstates tax if anyone is blind.
    BlindBoxForfeitedNotDeclared { per_box: Usd, persons: usize },
    /// ★ §164(b)(5) (P9 §2.2 / §3.4, r5 Nit-3): the sales-tax-instead-of-income-tax election was never
    /// asked, and a Schedule A exists — so SALT used income taxes. Fires on `salt_use_sales_tax.is_none()`
    /// ∧ `schedule_a.is_some()` (NOT "∧ the return itemizes", which would go silent exactly when the unasked
    /// election is what would flip the return into itemizing). Overstates tax if sales taxes are larger.
    /// `itemized` records which deduction the return took, so the text does not tell a standard-deduction
    /// filer their Schedule A "used" income taxes on a form they did not file (r3 MINOR-3, the r5 M-1 shape).
    SalesTaxElectionNotAsked { itemized: bool },
    /// ★★★ **§1411(c)(1)(B) / Form 8960 line 9b (P8, §3.4)** — the return owes net investment income
    /// tax and claims NOTHING on Part II line 9b, while the Schedule A it filed did deduct state and
    /// local income tax. The whole allocable deduction is forgone, which can only OVERSTATE the tax.
    ///
    /// ★★ It advises rather than computing, and that is the design (plan decision 6 /
    /// `ADJUDICATION-2026-08-21.md` D5's build-shape guard 1). i8960: *"You can determine the portion
    /// of your state, local, and foreign income taxes allocable to net investment income using **any
    /// reasonable method**"*, and *"the reasonable method of allocation may differ from year to
    /// year."* Choosing one is the filer's election; btctax states the POOL and offers the
    /// instructions' own worked example, and lets them enter the result.
    ///
    /// `bound` is the §164(b)(6)-limited pool from [`crate::tax::return_1040::nii_line9b_bound`] — the
    /// same number the `Nii9bExceedsDeductedSalt` refusal enforces, so the note and the gate can never
    /// name different figures. Fires only when `bound > 0`: a standard-deduction filer, or one who
    /// elected general sales taxes, forgoes nothing here and is never nagged.
    /// `bound` — the largest allocation §164(b)(6) leaves room for. `saving` — what claiming the
    /// whole of it would actually take off the tax, computed through Form 8960 line 16's own `min`
    /// (final whole-branch review, P3-3). The advisory is not emitted at all when `saving` is zero,
    /// because line 15 then binds and no allocation moves the return.
    Form8960Line9bNotClaimed { bound: Usd, saving: Usd },
    /// ★★★ **N1** — this year's capital loss was NOT absorbed by the §1211(b) allowance (in whole or
    /// in part), because taxable income was already at or below zero. The §1211/§1212 **Capital Loss
    /// Carryover Worksheet** therefore carries MORE loss into next year than the flat
    /// `loss − $3,000` rule does, and the filer has to be given the right number.
    ///
    /// ★★ **It exists because two figures for one quantity now coexist, and only one of them is
    /// right.** `report --tax-year` prints the frozen crypto-delta engine's `carryforward_out`, which
    /// is the flat rule and cannot be anything else — `TaxProfile` hands that engine an
    /// `ordinary_taxable_income` already floored at zero, so the worksheet's line 1 (1040 line 15
    /// *before* the floor) does not survive into it. A silent second number would be the worst of
    /// both worlds, so this advisory names BOTH and says which one to carry.
    ///
    /// ★ **Fires off the MECHANISM, not off a household list**: exactly when the worksheet's answer
    /// differs from the flat rule on either character. At positive taxable income the two are
    /// algebraically identical, so it cannot fire there — which is also what makes it a live test of
    /// the no-op claim rather than a restatement of it.
    CapitalLossCarryoverWorksheetIncreasesCarryover {
        /// The flat `loss − min(loss, §1211(b) limit)` figure, short-term character.
        flat_short: Usd,
        /// The flat figure, long-term character.
        flat_long: Usd,
        /// Worksheet line 8 — the short-term carryover.
        worksheet_short: Usd,
        /// Worksheet line 13 — the long-term carryover.
        worksheet_long: Usd,
        /// Worksheet line 4 — how much of the §1211(b) allowance the year actually absorbed. Zero
        /// when the loss did nothing at all for the filer this year.
        absorbed: Usd,
    },
}

/// Format a dollar amount for advisory prose: `$1,950` / `$1,234.56` — thousands-separated, and
/// cents shown only when there are any. [★ P5-N5] The advisories used a bare `{:.0}` (`$1950`),
/// which disagreed with the comma-separated house style every other printed figure uses. The CLI's
/// `fmt_money` lives in `btctax-cli::render` and core cannot reach it, so this is the core-side
/// equivalent — deliberately small, and used by every advisory that prints money.
pub(crate) fn fmt_usd(v: Usd) -> String {
    let cents = v.round_dp(2);
    let whole = cents.trunc().abs();
    let frac = (cents - cents.trunc()).abs();

    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    let sign = if cents.is_sign_negative() { "-" } else { "" };
    if frac.is_zero() {
        format!("{sign}${grouped}")
    } else {
        // `frac` is < 1; take its two decimal places without re-rounding.
        format!("{sign}${grouped}.{:02}", (frac * dec!(100)).round())
    }
}

impl Advisory {
    /// The user-facing text. Single-sourced here so the CLI, the man page and any future surface agree.
    pub fn message(&self) -> String {
        match self {
            Advisory::CtcOdcOmitted {
                dependents,
                provably_zero: true,
            } => format!(
                "CTC/ODC NOT COMPUTED, AND NOT AVAILABLE TO YOU — you captured {dependents} \
                 dependent(s), and v1 does not compute the Child Tax Credit or the Credit for Other \
                 Dependents. Here that costs you NOTHING: your income phases the credit out entirely \
                 under §24(b), whatever your dependents' ages (Schedule 8812 line 11 already exceeds \
                 the most line 8 could be). 1040 line 19 is $0 and that is the correct figure — there \
                 is no Schedule 8812 for you to file."
            ),
            Advisory::CtcOdcOmitted { dependents, .. } => format!(
                "CTC/ODC NOT COMPUTED — you captured {dependents} dependent(s), but v1 does not compute the \
                 Child Tax Credit or the Credit for Other Dependents (1040 line 19 is $0). Your tax is \
                 OVERSTATED by up to $2,000 per qualifying child / $500 per other dependent. File Schedule \
                 8812 yourself to claim it."
            ),
            Advisory::EicOmitted =>
                "EIC NOT COMPUTED — your income is low enough that you may qualify for the Earned Income \
                 Credit, which v1 does not compute. Your tax may be OVERSTATED. Check Pub. 596."
                    .to_string(),
            Advisory::AgedBoxForfeitedNoDob { per_box } => format!(
                "DATE OF BIRTH NOT ON FILE — the §63(f) additional standard deduction for age 65+ \
                 ({} per box) was NOT granted, because v1 never assumes a birthdate. If you (or your \
                 spouse) are 65 or older, enter the date of birth and re-run: your tax is currently \
                 OVERSTATED.",
                fmt_usd(*per_box)
            ),
            Advisory::AgedBoxForfeitedDeathUnanswered { per_box, persons } => format!(
                "AGE-65 BOX FORGONE — a date of birth on file qualifies {n} for the §63(f) additional \
                 standard deduction ({amt} per box), but the \"died during the tax year?\" question was \
                 not answered, so it was NOT granted. i1040gi carves out someone who died in-year \
                 before reaching 65, and v1 will not resolve that carve-out by assuming they lived. \
                 Answering it with `btctax income answer` is one keystroke and worth {total}: your tax \
                 is currently OVERSTATED.",
                n = if *persons > 1 {
                    "you and your spouse"
                } else {
                    "someone on this return"
                },
                amt = fmt_usd(*per_box),
                // ★ The VALUE of answering is the whole forfeit, not one box. `persons` already
                // selects the pronoun; quoting the per-box figure beside "you and your spouse" told an
                // MFJ couple $1,550 when $3,100 was at stake. A wrong number in the text that exists
                // to make the number vivid is worse than no number.
                total = fmt_usd(*per_box * Usd::from(*persons as u64))
            ),
            Advisory::QbiCarryforwardNotStated =>
                "PRIOR-YEAR QBI LOSS CARRYFORWARD NOT STATED — this return claims a §199A qualified \
                 business income deduction, and btctax has no prior-year loss carryforward on file for \
                 it (Form 8995 lines 7 and 3). Those lines carry LOSSES that REDUCE the deduction, so \
                 if you had one and it is not entered, your deduction is too large and your tax is \
                 UNDERSTATED — the one direction btctax will not fail in silently. Check lines 16 and \
                 17 of LAST year's Form 8995: if either is non-zero, enter it here as a POSITIVE \
                 amount. If you had no §199A activity last year, or last year's lines 16 and 17 were \
                 zero, there is nothing to do and this note is expected."
                    .to_string(),
            Advisory::ExcessSsNotCreditable { whose, ein, amount } => format!(
                "SOCIAL SECURITY OVER-WITHHELD BY ONE EMPLOYER — employer EIN {ein} withheld {} more \
                 than the §3101(a) cap from {whose}. §6413(c) does not let you claim THAT employer's \
                 excess on this return, however many employers {whose} had, so Schedule 3 line 11 \
                 omits it and that is correct. The money is still yours: ask that employer to adjust \
                 the overcollection, and if they don't, file a claim for refund using Form 843.",
                fmt_usd(*amount)
            ),
            Advisory::Mfs63fSpouseBoxesForgone { per_box, boxes } => format!(
                "SPOUSE'S §63(f) BOX{p} NOT CLAIMED ON A SEPARATE RETURN — you told btctax something \
                 that would qualify your spouse for {n} additional standard-deduction box{p} ({amt} \
                 each, {total} in total). On married-filing-separately the instructions allow them \
                 \"if your spouse had no income, isn't filing a return, and can't be claimed as a \
                 dependent on another person's return\" — and btctax has not been told all three, so \
                 it does not claim them. Your tax is OVERSTATED by {total} if all three are true. \
                 ★ ANSWER THEM RATHER THAN CHECKING THE BOX{p} BY HAND: set `spouse_had_no_income` and \
                 `spouse_not_filing_a_return` (via `btctax income import`, or `btctax income answer` \
                 for the dependent question) and btctax will check the box{p} AND raise the standard \
                 deduction together. Checking {they} by hand moves the box count without moving line \
                 12, and the two must agree.",
                n = boxes,
                p = if *boxes > 1 { "ES" } else { "" },
                they = if *boxes > 1 { "them" } else { "it" },
                amt = fmt_usd(*per_box),
                total = fmt_usd(*per_box * Usd::from(*boxes as u64))
            ),
            Advisory::BenefitCarryoversNotStated {
                capital_loss,
                charitable,
            } => format!(
                "PRIOR-YEAR CARRYOVER{p} NOT STATED — btctax has no {which} on file, and it did not \
                 compute your prior year, so it cannot tell \"you have none\" from \"nobody asked\". \
                 {p2} REDUCE your tax, so leaving {one} out costs YOU, not the Treasury — btctax will \
                 not invent {one}, and your tax may be OVERSTATED. Check last year's return and enter \
                 {one} with `btctax income import` if you have {one}.",
                p = if *capital_loss && *charitable { "S" } else { "" },
                which = match (capital_loss, charitable) {
                    (true, true) =>
                        "capital-loss carryover (§1212(b), Schedule D lines 6/14) or charitable \
                         carryover (§170(d)(1))",
                    (true, false) => "capital-loss carryover (§1212(b), Schedule D lines 6/14)",
                    _ => "charitable carryover (§170(d)(1))",
                },
                p2 = if *capital_loss && *charitable {
                    "Both"
                } else {
                    "It would"
                },
                one = if *capital_loss && *charitable {
                    "them"
                } else {
                    "it"
                }
            ),
            Advisory::FbarFinCen =>
                "FBAR / FinCEN — you declared a foreign financial account. Under FinCEN Notice 2020-2 an \
                 account holding ONLY virtual currency is (for now) outside the FBAR requirement, but that \
                 is under active reconsideration, and an account holding crypto PLUS fiat or securities may \
                 well be reportable. btctax never answers Schedule B Part III for you — decide it yourself."
                    .to_string(),
            Advisory::ScheduleC1099NotAnswered { line_j_too } => format!(
                "SCHEDULE C FORM-1099 QUESTION{} LEFT BLANK — {}. That is lawful: no figure on your \
                 return reads {}, and btctax will never answer for you. But §6721 (failure to file a \
                 required information return) and §6722 (failure to furnish the payee's copy) apply to \
                 the PAYMENTS, not to this box — leaving it blank neither creates nor removes that \
                 exposure. If you paid $600 or more to a contractor or service provider for your \
                 business, check the Schedule C instructions and answer with `btctax income answer`.",
                if *line_j_too { "S" } else { "" },
                if *line_j_too {
                    "you answered line I \"Yes\" but did not answer line J (\"did you or will you \
                     file required Form(s) 1099?\"), so BOTH go out blank — and you have already \
                     declared the payments exist"
                } else {
                    "line I (\"did you make any payments that would require you to file Form(s) \
                     1099?\") was not answered, so the box goes out blank"
                },
                if *line_j_too { "them" } else { "it" }
            ),
            Advisory::FbarSubQuestionNotAnswered =>
                "FBAR SUB-QUESTION LEFT BLANK — you answered Schedule B line 7a \"Yes\" but did not answer \
                 its sub-question (\"are you required to file FinCEN Form 114?\"), so that box prints \
                 BLANK. That is lawful: no figure on your return reads it, and btctax will never answer \
                 it for you. But read Schedule B's own Caution first — \"If required, failure to file \
                 FinCEN Form 114 may result in substantial penalties. Additionally, you may be required \
                 to file Form 8938, Statement of Specified Foreign Financial Assets.\" That penalty is \
                 for not FILING the FBAR, an obligation this box does not create or remove. Answer it \
                 with `btctax income answer` if you want the box completed."
                    .to_string(),
            Advisory::CharitableDoneeAssumedPublicCharity { donations } => format!(
                "CHARITABLE DONEE ASSUMED — your {donations} crypto donation(s) were valued assuming a \
                 PUBLIC CHARITY (50%-organization) donee: long-term gifts at fair market value under the \
                 30%-of-AGI ceiling. If the donee is a PRIVATE FOUNDATION, the correct treatment is the \
                 20% ceiling at BASIS (which v1 refuses). Verify who you gave to."
            ),
            Advisory::QualifiedAppraisalMustBeAttached { claimed } => format!(
                "ATTACH THE QUALIFIED APPRAISAL — you are claiming {} of charitable deduction for \
                 donated property, and §170(f)(11)(D) says that \"in the case of contributions of \
                 property for which a deduction of more than $500,000 is claimed\", the substantiation \
                 requirements are met only if you ATTACH a qualified appraisal to the return. This is \
                 more than the $5,000 rule, which only asks you to obtain one and keep it: over \
                 $500,000 the appraisal is part of the filed return, and without it the deduction can \
                 be denied in full. btctax cannot write an appraisal — only a qualified appraiser \
                 can — so obtain it and staple it to the return ON WHICH YOU CLAIM THE DEDUCTION \
                 (manifest.txt lists it whenever this packet is that return). ★ The duty RECURS: \
                 Reg §1.170A-16(f)(3) requires the appraisal attached to the return for any \
                 §170(d) carryover year too, so attach a copy again in every year this gift carries \
                 into.",
                fmt_usd(*claimed)
            ),
            Advisory::OtherCreditsOmitted =>
                "OTHER CREDITS NOT COMPUTED — v1 does not compute the education (Form 8863), \
                 dependent-care (Form 2441), retirement-savings/saver's (Form 8880), residential-energy \
                 (Form 5695) or adoption (Form 8839) credits: Schedule 3 Part I is $0 apart from the \
                 foreign tax credit. If you qualify for any of them your tax is OVERSTATED — claim them \
                 yourself."
                    .to_string(),
            Advisory::RefundByPaperCheck { refund } => format!(
                "REFUND BY PAPER CHECK — your return is due a refund of {}, but v1 never fills the \
                 direct-deposit block (1040 lines 35b–35d). As filed, the IRS will mail a check. Add your \
                 routing and account numbers by hand if you want it deposited.",
                fmt_usd(*refund)
            ),
            // ★ §3.4 (r5 M-1): the text branches on the deduction actually taken. The itemized filer filed a
            // Schedule A with a $0 line 8a and a checked box; the standard filer filed NO Schedule A, so the
            // note must not describe one. `forgone_interest` is a CEILING ("up to"), never "the amount lost".
            Advisory::MixedUseMortgageNotAllocated {
                forgone_interest,
                itemized,
            } => {
                if *itemized {
                    format!(
                        "MIXED-USE MORTGAGE — Your Schedule A claimed $0 on line 8a and the mixed-use box is \
                         checked. Because not all of the loan was used to buy, build, or improve the home, \
                         §163(h)(3)(F) makes the rest non-deductible and v1 cannot compute the Pub. 936 \
                         allocation. A Pub. 936 allocation could restore up to {} of mortgage interest — your \
                         tax is OVERSTATED.",
                        fmt_usd(*forgone_interest)
                    )
                } else {
                    format!(
                        "MIXED-USE MORTGAGE — Your return took the standard deduction. Because you declared a \
                         mixed-use mortgage, line 8a was treated as $0 (§163(h)(3)(F); v1 cannot compute the \
                         Pub. 936 allocation); a Pub. 936 allocation of up to {} of mortgage interest might \
                         have made itemizing win.",
                        fmt_usd(*forgone_interest)
                    )
                }
            }
            Advisory::BlindBoxForfeitedNotDeclared { per_box, persons } => format!(
                "BLINDNESS NOT DECLARED — the §63(f) additional standard deduction for blindness ({} per \
                 box) was NOT granted for {persons} person(s) whose blindness was never stated (v1 never \
                 assumes it). It STACKS with the age-65+ box. If you (or your spouse) are legally blind, run \
                 `btctax income answer`: your tax is currently OVERSTATED.",
                fmt_usd(*per_box)
            ),
            // ★ r3 MINOR-3 — branch on the deduction actually taken (the r5 M-1 shape): the itemized filer
            // filed a Schedule A that used income taxes; the standard filer filed none, so the text must not
            // say "your Schedule A used …".
            Advisory::SalesTaxElectionNotAsked { itemized } => {
                if *itemized {
                    "SALES-TAX ELECTION NOT ASKED — your Schedule A used state and local INCOME taxes, but \
                     you were never asked whether to deduct general SALES taxes instead (§164(b)(5)). In a \
                     no-income-tax state or a big-purchase year the sales-tax figure can be larger. If so, \
                     your SALT deduction is too small and your tax is OVERSTATED. Run `btctax income answer` \
                     to choose."
                        .to_string()
                } else {
                    "SALES-TAX ELECTION NOT ASKED — you have Schedule A items but took the standard \
                     deduction, and were never asked whether to deduct general SALES taxes instead of \
                     income taxes (§164(b)(5)). In a no-income-tax state or a big-purchase year the \
                     sales-tax figure can be larger — and could even flip this return into itemizing. If \
                     so, your tax is OVERSTATED. Run `btctax income answer` to choose."
                        .to_string()
                }
            }
            // ★ P8 — the pool is named, the METHOD is not applied. i8960's own example is offered as
            //   an example ("one reasonable method"), never as the answer, because "any reasonable
            //   method" is the filer's election and may differ from year to year.
            Advisory::Form8960Line9bNotClaimed { bound, saving } => format!(
                "FORM 8960 LINE 9B NOT CLAIMED — you owe net investment income tax (§1411), and \
                 Form 8960 line 9b, \"State, local, and foreign income tax\", is BLANK. The state \
                 and local income tax your Schedule A actually deducted, after §164(b)(6)'s limit, \
                 is up to {}, and the portion of it attributable to your investment income is \
                 deductible against that income — so your tax is currently OVERSTATED by up to {}. \
                 (That is what allocating the WHOLE of it would save; a smaller allocation saves \
                 less. Line 16 is the smaller of line 12 and line 15, so once line 12 falls to line \
                 15 no further allocation changes anything.) btctax will \
                 not pick the split for you: the Instructions for Form 8960 say you may use \"any \
                 reasonable method\", and one they give themselves is that amount times the ratio of \
                 Form 8960 line 8 (gross investment income) to your AGI. Work out your own figure and \
                 enter it, or leave the line blank and claim nothing.",
                fmt_usd(*bound),
                fmt_usd(*saving)
            ),
            // ★ F3 (phase-1 seam review). This said the flat figure was "printed above", which is
            //   false on one surface and can be false on both. §G-19d prints this same advisory on
            //   `export-irs-pdf` STDERR, where no carryforward figure is printed above it at all.
            //   And `flat` here is `capital_net`'s full-return netting (broker 1099-B totals joined),
            //   while the figure `report` actually prints is the delta engine's CRYPTO-ONLY
            //   carryforward — so on a floor household with broker capital losses it pointed at a
            //   number appearing nowhere on the page. An advisory must not tell a filer to compare
            //   against something it cannot guarantee is in front of them; naming the RULE is true on
            //   every surface. The bottom-line instruction (carry the worksheet figure) is unchanged.
            Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover {
                flat_short,
                flat_long,
                worksheet_short,
                worksheet_long,
                absorbed,
            } => format!(
                "CAPITAL-LOSS CARRYOVER — CARRY {ws}, NOT {flat}. Your taxable income (Form 1040 line \
                 15) was already at or below zero, so the capital loss did not offset ordinary income \
                 the way it does in a normal year: the §1211(b) allowance absorbed only {absorbed} of \
                 it. The §1211/§1212 Capital Loss Carryover Worksheet (2025 Schedule D instructions, \
                 \"Capital Loss Carryover Worksheet — Lines 6 and 14\") therefore carries {ws} into \
                 next year — short-term {ws_s}, long-term {ws_l} — where the flat \"loss minus \
                 $3,000\" rule gives {flat}. ★ THE WORKSHEET FIGURE IS THE CORRECT ONE. \
                 Enter it on next year's Schedule D lines 6 and 14, and as next year's capital-loss \
                 carryover in `btctax income import`; the flat figure would forfeit {diff} of \
                 deductible loss permanently.",
                ws = fmt_usd(*worksheet_short + *worksheet_long),
                flat = fmt_usd(*flat_short + *flat_long),
                ws_s = fmt_usd(*worksheet_short),
                ws_l = fmt_usd(*worksheet_long),
                absorbed = fmt_usd(*absorbed),
                diff = fmt_usd(
                    (*worksheet_short + *worksheet_long) - (*flat_short + *flat_long)
                ),
            ),
        }
    }
}

/// Collect every advisory that applies to a computed full return for `year`, from the assembled return.
/// Is the §24 credit ZERO for this filer whatever their dependents' ages are?
///
/// ★★★ Transcribed from **Schedule 8812 (Form 1040) Part I**, not derived. btctax does not compute the
/// credit and does not know which dependents are under 17, so it cannot compute lines 4/6 — but it can
/// compute the CEILING of line 8 and compare it to line 11, and when the ceiling loses, the credit is
/// zero for every possible composition. That is a sound one-sided answer, which is exactly what an
/// advisory needs.
///
/// ```text
///  1  Enter the amount from line 11 of your Form 1040 …
/// 2a  Enter income from Puerto Rico that you excluded
///  b  Enter the amounts from lines 45 and 50 of your Form 2555
///  c  Enter the amount from line 15 of your Form 4563
/// 2d  Add lines 2a through 2c
///  3  Add lines 1 and 2d
///  5  Multiply line 4 by $2,000            ← line 4 = qualifying children under 17
///  7  Multiply line 6 by $500              ← line 6 = other dependents
///  8  Add lines 5 and 7
///  9  Enter the amount shown below for your filing status.
///     • Married filing jointly—$400,000
///     • All other filing statuses—$200,000
/// 10  Subtract line 9 from line 3.
///     • If zero or less, enter -0-.
///     • If more than zero and not a multiple of $1,000, enter the next multiple of $1,000. …
/// 11  Multiply line 10 by 5% (0.05)
/// 12  Is the amount on line 8 more than the amount on line 11?
/// ```
///
/// Line 12 answering **No** is the whole credit gone. The ceiling of line 8 is `dependents × $2,000`
/// (every dependent a qualifying child — $500 for an "other dependent" can only be smaller), so
/// `ceiling ≤ line 11` proves it. ★ btctax already collects every line-2 add-back, so line 3 is exact,
/// not approximated by AGI.
fn ctc_provably_zero(ri: &ReturnInputs, dependents: usize, agi: Usd) -> bool {
    // ★★★ L3 IS A LOWER BOUND, and a bound is all this predicate needs.
    //
    // This used `modified_agi`, whose `None` (gate never asked) made it return false. Correct in
    // isolation — but `HasIncomeExclusion` is `live: |ri| ri.tax_year >= 2025`, so on TY2024, the only
    // year btctax can file, the gate is NEVER ASKED and `modified_agi` is ALWAYS `None`. The whole
    // provable-zero branch was therefore **dead in production**, while its tests passed by setting the
    // gate by hand. A filer with nine children at $2.08M AGI was still told their tax was overstated
    // by up to $18,000 and sent to Schedule 8812 for a credit §24(b) had entirely removed — the exact
    // defect the branch was written to fix, still shipping.
    //
    // ★★ MONOTONICITY is what makes a bound sufficient: L11 = 5% × ceil(L3 − L9) is non-decreasing in
    //    L3, and the test is `L8_ceiling ≤ L11`. So if the credit is provably gone at a LOWER BOUND on
    //    MAGI, it is gone at the true MAGI too. No answer to the gate is required, and none is assumed.
    //
    // ★ The bound stays conservative in the direction that costs a filer money: an unblessed exclusion
    //   yields `agi`, the proof fails, and they are told to check Schedule 8812 (r8 F2), never the
    //   reverse. See `ReturnInputs::modified_agi_lower_bound` for the two-branch argument.
    // ★★ EXACT WHEN KNOWN, BOUNDED WHEN NOT — and the first draft of this fix got it wrong by using
    //    the bound unconditionally, which threw away add-backs the filer HAD blessed. A gate answered
    //    `true` with $20,000 of excluded Puerto Rico income is testimony that raises MAGI and can
    //    legitimately complete the proof; discarding it is conservative in the harmless direction but
    //    it is still discarding an answer the filer gave. `the_line_2_add_backs_count_but_only_once_
    //    the_gate_is_answered` caught it.
    let l3 = ri
        .modified_agi(agi)
        .unwrap_or_else(|| ri.modified_agi_lower_bound(agi));
    // L9.
    let l9 = if ri.filing_status == crate::tax::types::FilingStatus::Mfj {
        Usd::from(400_000)
    } else {
        Usd::from(200_000)
    };
    // L10 — "If zero or less, enter -0-. If more than zero and not a multiple of $1,000, enter the
    // NEXT multiple of $1,000." Rounding UP is the filer-adverse direction and the form says so.
    let over = (l3 - l9).max(Usd::ZERO);
    if over.is_zero() {
        return false;
    }
    let thousands = (over / Usd::from(1000)).ceil();
    let l11 = thousands * Usd::from(1000) * rust_decimal_macros::dec!(0.05); // L11
    let l8_ceiling = Usd::from(dependents as i64) * Usd::from(2000);
    // L12 "Is the amount on line 8 more than the amount on line 11?" — No ⇒ no credit.
    l8_ceiling <= l11
}

#[cfg(test)]
mod ctc_phaseout_tests {
    use super::*;
    use crate::tax::types::FilingStatus;

    fn ri_with(status: FilingStatus, deps: usize) -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: status,
            // ★ ANSWERED, because these cases are about the PHASE-OUT arithmetic, not the gate. An
            //   unanswered gate makes MAGI unknown and `ctc_provably_zero` correctly declines to prove
            //   anything — see `the_line_2_add_backs_count_but_only_once_the_gate_is_answered`.
            has_income_exclusion: Some(false),
            ..Default::default()
        };
        for i in 0..deps {
            ri.header
                .dependents
                .push(crate::tax::return_inputs::Dependent {
                    name: format!("Child {i}"),
                    ..Default::default()
                });
        }
        ri
    }

    /// ★★★ The filing trial's own vector: AGI $2,085,000, MFJ, NINE children. The advisory told this
    /// filer their tax was overstated by up to $18,000 and sent them to Schedule 8812 — while §24(b)
    /// had already reduced the credit by $84,250. Line 19 was $0 and correct; the ADVICE was not.
    #[test]
    fn nine_children_at_two_million_agi_is_provably_zero() {
        let ri = ri_with(FilingStatus::Mfj, 9);
        assert!(ctc_provably_zero(&ri, 9, Usd::from(2_085_000)));
    }

    /// ★★★ THE PRODUCTION STATE — the gate UNANSWERED, which is every TY2024 return.
    ///
    /// `HasIncomeExclusion` is `live: |ri| ri.tax_year >= 2025`, so on the only year btctax can file
    /// it is never asked and `has_income_exclusion` stays `None`. Every other test in this module sets
    /// it by hand, which is legitimate for exercising the phase-out arithmetic but meant the
    /// provable-zero branch was **never exercised as production reaches it** — and in production it
    /// could not be reached at all. This is the test that distinguishes the two.
    #[test]
    fn the_gate_being_unasked_does_not_defeat_the_proof() {
        let mut ri = ri_with(FilingStatus::Mfj, 9);
        ri.has_income_exclusion = None; // TY2024, as shipped
        assert_eq!(
            ri.modified_agi(Usd::from(2_085_000)),
            None,
            "gate unanswered"
        );
        assert!(
            ctc_provably_zero(&ri, 9, Usd::from(2_085_000)),
            "a LOWER BOUND proves the phase-out; the gate's answer cannot lower MAGI, only raise it"
        );
    }

    /// ★★ …and the r8 F2 protection survives: an UNBLESSED exclusion must never produce a false
    /// "you get nothing". The bound ignores positive add-backs, so the proof simply fails — the filer
    /// is told to check Schedule 8812 rather than talked out of money they may be owed.
    #[test]
    fn an_unblessed_exclusion_never_manufactures_a_provable_zero() {
        let mut ri = ri_with(FilingStatus::Mfj, 2);
        ri.has_income_exclusion = None;
        ri.form_2555_line45 = Usd::from(200_000);
        // True MAGI *might* be 600,000 (enough to kill a 2-child credit), but nothing blessed it.
        assert!(
            !ctc_provably_zero(&ri, 2, Usd::from(400_000)),
            "an unasked exclusion must not be counted toward proving the credit gone"
        );
    }

    /// ★★ A NEGATIVE add-back cannot inflate the bound. The four fields are exclusion amounts and a
    /// negative is nonsense, but `first_negative_amount` explicitly waives them, so the bound is
    /// written to survive one instead of assuming it away.
    #[test]
    fn a_negative_add_back_only_lowers_the_bound() {
        let mut ri = ri_with(FilingStatus::Mfj, 9);
        ri.has_income_exclusion = None;
        ri.form_2555_line45 = Usd::from(-50_000);
        assert_eq!(
            ri.modified_agi_lower_bound(Usd::from(2_085_000)),
            Usd::from(2_035_000),
            "a negative is added back, never clamped away — the bound must stay a bound"
        );
        assert!(
            ctc_provably_zero(&ri, 9, Usd::from(2_085_000)),
            "still gone"
        );
    }

    /// …and it must NOT claim provable-zero when the credit is genuinely available. A wrong "you get
    /// nothing" is the worse error of the two: it talks a filer out of money they are owed.
    #[test]
    fn an_ordinary_household_is_not_told_the_credit_is_gone() {
        let ri = ri_with(FilingStatus::Mfj, 2);
        assert!(!ctc_provably_zero(&ri, 2, Usd::from(90_000)));
        // Exactly AT the threshold: L10 is -0-, so nothing is phased out at all.
        assert!(!ctc_provably_zero(&ri, 2, Usd::from(400_000)));
    }

    /// ★★ THE BOUNDARY, worked from the form. MFJ, 2 dependents ⇒ line 8 ceiling $4,000. The credit
    /// survives while line 11 < 4,000, i.e. line 10 < 80,000, i.e. excess ≤ 79,000 (line 10 rounds UP
    /// to the next $1,000). So $479,000 still has a credit and $479,001 does not.
    #[test]
    fn the_phase_out_boundary_is_the_forms_own_rounding() {
        let ri = ri_with(FilingStatus::Mfj, 2);
        assert!(
            !ctc_provably_zero(&ri, 2, Usd::from(479_000)),
            "excess 79,000 -> L10 79,000 -> L11 3,950 < 4,000 ceiling: credit survives"
        );
        assert!(
            ctc_provably_zero(&ri, 2, Usd::from(479_001)),
            "excess 79,001 ROUNDS UP to L10 80,000 -> L11 4,000, which is not LESS than the 4,000 \
             ceiling, so line 12 answers No"
        );
    }

    /// The single/HoH threshold is half the joint one, and the code must not assume MFJ.
    #[test]
    fn the_threshold_follows_filing_status() {
        let single = ri_with(FilingStatus::Single, 1);
        assert!(ctc_provably_zero(&single, 1, Usd::from(240_001)));
        let mfj = ri_with(FilingStatus::Mfj, 1);
        assert!(
            !mfj_is_zero(&mfj),
            "the same income is nowhere near the joint threshold"
        );
    }
    fn mfj_is_zero(ri: &ReturnInputs) -> bool {
        ctc_provably_zero(ri, 1, Usd::from(240_001))
    }

    /// ★ Line 3 is MODIFIED AGI — the excluded-income add-backs push a filer over. They count only
    /// when the §911/931/933 GATE says they exist.
    #[test]
    fn the_line_2_add_backs_count_but_only_once_the_gate_is_answered() {
        let mut ri = ri_with(FilingStatus::Mfj, 1);
        ri.has_income_exclusion = Some(true);
        assert!(!ctc_provably_zero(&ri, 1, Usd::from(430_000)));
        ri.excluded_puerto_rico_income = Usd::from(20_000);
        assert!(
            ctc_provably_zero(&ri, 1, Usd::from(430_000)),
            "line 3 = line 1 + line 2d, so excluded Puerto Rico income phases the credit out"
        );

        // ★★★ r8 F2 — THE SAME AMOUNTS WITH THE GATE UNANSWERED PROVE NOTHING. On TY2024 the gate is
        //     never asked while the amount fields are always-live inputs, so this is the SHIPPED shape,
        //     not a contrived one. Claiming the credit is gone here tells a filer owed $2,000 that
        //     "there is no Schedule 8812 for you to file".
        let mut ungated = ri_with(FilingStatus::Mfj, 1);
        // Re-blank the gate: `ri_with` answers it, and THIS case is precisely the unanswered one.
        ungated.has_income_exclusion = None;
        ungated.form_2555_line45 = Usd::from(200_000);
        assert!(
            !ctc_provably_zero(&ungated, 1, Usd::from(296_500)),
            "MAGI is UNKNOWN when the gate is unanswered — an advisory may not claim a benefit is \
             gone on a number nobody confirmed"
        );

        // …and Some(false) means the add-backs are genuinely zero, so MAGI == AGI and the ordinary
        // arithmetic applies. Without this the fix could pass by never proving anything at all.
        let mut none_stated = ri_with(FilingStatus::Mfj, 1);
        none_stated.has_income_exclusion = Some(false);
        assert!(ctc_provably_zero(&none_stated, 1, Usd::from(500_000)));
        assert!(!ctc_provably_zero(&none_stated, 1, Usd::from(296_500)));
    }
}

pub fn advisories_for(
    ri: &ReturnInputs,
    state: &LedgerState,
    ar: &AbsoluteReturn,
    params: &FullReturnParams,
    year: i32,
) -> Vec<Advisory> {
    let earned = ar.wages + ar.se.as_ref().map_or(Usd::ZERO, |s| s.net_se);
    let mut out = advisories(
        ri,
        state,
        earned,
        ar.agi,
        ar.overpayment_refund,
        params,
        year,
        ar.deduction_is_itemized,
    );
    // ★★★ **P8 / §3.4** — Form 8960 Part II line 9b is blank on a return that OWES NIIT and did
    //     deduct state income tax. Fires here rather than in `advisories`, because both halves of the
    //     predicate need the COMPUTED return: whether §1411 tax is owed at all, and the §63(e)
    //     election the bound depends on.
    //
    // ★ Gated on `bound > Usd::ZERO` so it stays silent where nothing is forgone — the standard
    //   deduction (nothing was "properly deducted on your return") and the §164(b)(5) sales-tax
    //   election (i8960: "Sales taxes aren't deductible in computing net investment income"). Both
    //   are real branches of the same derivation the refusal uses, not exclusions bolted on here.
    //
    // ★★★ …AND GATED ON WHETHER IT WOULD ACTUALLY MOVE THE TAX (final whole-branch review, P3-3).
    //
    //     The text said "so your tax is currently OVERSTATED", unconditionally. That is FALSE
    //     whenever LINE 15 BINDS. Form 8960 line 16 is `min(line 12, line 15)`, so when the
    //     MAGI excess (line 15) is smaller than net investment income (line 12) — wages $150,000
    //     plus $100,000 of crypto gains: excess $50,000 < NII $100,000, a COMMON btctax shape —
    //     line 16 takes the line-15 leg and a 9b entry of any size up to the bound changes the tax
    //     by exactly $0. The advisory told that filer they were overpaying when they were not, and
    //     invited a sworn allocation election that buys them nothing.
    //
    //     So the benefit is COMPUTED from the form's own arithmetic rather than asserted:
    //     `min(l12, l15) − min(max(l12 − bound, 0), l15)`, times the rate. Reducing line 12 first
    //     eats the slack by which it exceeds line 15 and only then moves line 16 — which is the
    //     mechanism, so it decides, and no filer shape has to be enumerated.
    if ri.form_8960_line9b.is_none() && ar.niit.tax > Usd::ZERO {
        let bound = crate::tax::return_1040::nii_line9b_bound(ar);
        let l12 = ar.niit.nii;
        let l15 =
            (ar.niit.magi - crate::tax::tables::niit_threshold(ri.filing_status)).max(Usd::ZERO);
        let l16_now = l12.min(l15);
        let l16_after = (l12 - bound).max(Usd::ZERO).min(l15);
        let saving = crate::conventions::round_cents(
            crate::tax::tables::NIIT_RATE * (l16_now - l16_after).max(Usd::ZERO),
        );
        if bound > Usd::ZERO && saving > Usd::ZERO {
            out.push(Advisory::Form8960Line9bNotClaimed { bound, saving });
        }
    }

    // ★★★ **N1** — the §1211/§1212 Capital Loss Carryover Worksheet moved the carryforward, so say so.
    //
    // Fires off the MECHANISM — "the worksheet and the flat rule disagree" — never off a list of
    // households. At non-negative 1040 line 15 the two are algebraically the same number (pinned by
    // `capital_loss_carryover::at_nonnegative_line1_the_worksheet_equals_the_frozen_flat_rule`), so
    // this cannot fire on an ordinary loss year; it fires exactly on the floor region, which is
    // precisely the region no corpus household and neither oracle can witness.
    if let Some(w) = ar.capital_loss_carryover_worksheet {
        let flat = crate::tax::return_1040::capital_net(ri, state, year, ri.filing_status);
        let ws = w.carryforward_out();
        if ws.short != flat.st_carry || ws.long != flat.lt_carry {
            out.push(Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover {
                flat_short: flat.st_carry,
                flat_long: flat.lt_carry,
                worksheet_short: ws.short,
                worksheet_long: ws.long,
                absorbed: w.line4,
            });
        }
    }

    // ★★★ §6413(c) — computed on the return (it needs the year's wage base, which the scalar form does
    // not carry), appended here so the filer is TOLD that money withheld above the cap by a SINGLE
    // employer is real, recoverable, and simply not claimable on this return.
    for nc in &ar.excess_ss_not_creditable {
        out.push(Advisory::ExcessSsNotCreditable {
            whose: match nc.owner {
                crate::tax::return_inputs::Owner::Taxpayer => "you",
                crate::tax::return_inputs::Owner::Spouse => "your spouse",
            },
            ein: nc.ein.clone(),
            amount: nc.amount,
        });
    }
    out
}

/// Collect every advisory that applies (the scalar form — `earned_income` = wages + net SE earnings;
/// `agi` = 1040 L11; `refund` = 1040 L34/L35a, zero when the return owes; `deduction_is_itemized` is the
/// filed return's deduction choice, which the mixed-use-mortgage advisory branches its text on — §3.4).
/// Order is stable: omissions first (they cost the filer money), then disclosures.
#[allow(clippy::too_many_arguments)]
pub fn advisories(
    ri: &ReturnInputs,
    state: &LedgerState,
    earned_income: Usd,
    agi: Usd,
    refund: Usd,
    params: &FullReturnParams,
    year: i32,
    deduction_is_itemized: bool,
) -> Vec<Advisory> {
    let mut out = Vec::new();

    // §3.4 — CTC/ODC: captured dependents, but line 19 is $0.
    let dependents = ri.header.dependents.len();
    if dependents > 0 {
        out.push(Advisory::CtcOdcOmitted {
            dependents,
            provably_zero: ctc_provably_zero(ri, dependents, agi),
        });
    }

    // §3.4 — EIC: earned income present and AGI low enough that the household might qualify.
    if earned_income > Usd::ZERO && agi < EIC_ADVISORY_AGI_CEILING {
        out.push(Advisory::EicOmitted);
    }

    // [★ P5-I2] §3.4 / SPEC §1.2 — the other favorable credits v1 never computes. UNCONDITIONAL: v1
    // captures no input that could establish eligibility, so it cannot know whether this filer
    // qualifies, only that it did not try. LIMITATIONS.md promises every omission row fires an
    // advisory; before this, two of the four rows fired nothing at all.
    out.push(Advisory::OtherCreditsOmitted);

    // [★ P5-I2] SPEC §9.2 — no direct-deposit block is ever filled. Only actionable on a refund.
    if refund > Usd::ZERO {
        out.push(Advisory::RefundByPaperCheck { refund });
    }

    // §63(f) — a missing DOB forfeits the aged box (never granted on an unsubstantiated birthdate).
    let married_rate = matches!(
        ri.filing_status,
        FilingStatus::Mfj | FilingStatus::Mfs | FilingStatus::Qss
    );
    let per_box = if married_rate {
        params.std_aged_blind_married
    } else {
        params.std_aged_blind_unmarried
    };
    let taxpayer_no_dob = ri.header.taxpayer.date_of_birth.is_none();
    // [★ P5-M2] On MFJ, an ABSENT spouse record forfeits the spouse's §63(f) box just as surely as a
    // spouse record with no DOB does — `standard_deduction` only counts spouse boxes when the record
    // exists. The old `is_some_and(no dob)` returned false for `spouse: None`, so that forfeit was
    // silent, and nothing in `screen_inputs` requires a spouse record on MFJ. Absent ⇒ not on file.
    let spouse_dob_on_file = ri
        .header
        .spouse
        .as_ref()
        .is_some_and(|s| s.date_of_birth.is_some());
    let spouse_no_dob = crate::tax::questions::spouse_63f_status_permits(ri) && !spouse_dob_on_file;
    if taxpayer_no_dob || spouse_no_dob {
        out.push(Advisory::AgedBoxForfeitedNoDob { per_box });
    }

    // ★ §G-9: the OTHER way to forfeit the aged box — a qualifying DOB is on file but the death
    // question was skipped, so `is_aged` forgoes rather than resolve the carve-out by assuming the
    // person lived. Counted per person; the spouse term asks `spouse_63f_boxes_count`, i.e. exactly
    // when a spouse box was on the table to lose (r3 I-1 — this said "only on MFJ", which `fd9c15f`
    // falsified when it made the boxes claimable on a qualifying MFS return).
    let forgone = |p: &crate::tax::return_inputs::Person, died: Option<bool>| {
        crate::tax::return_1040::aged_box_forgone_for_unanswered_death(
            p.date_of_birth,
            died,
            p.date_of_death,
            year,
        )
    };
    let death_forgone_persons = usize::from(forgone(
        &ri.header.taxpayer,
        ri.header.taxpayer_died_during_year,
    )) + usize::from(
        crate::tax::questions::spouse_63f_status_permits(ri)
            && ri
                .header
                .spouse
                .as_ref()
                .is_some_and(|s| forgone(s, ri.header.spouse_died_during_year)),
    );
    if death_forgone_persons > 0 {
        out.push(Advisory::AgedBoxForfeitedDeathUnanswered {
            per_box,
            persons: death_forgone_persons,
        });
    }

    // ★ §63(f) BLINDNESS forgone (P9 §2.2) — same statute, rate and worksheet line as the aged box, and it
    // STACKS. Fires on `blind.is_none()` (never asked), never on `Some(false)`, counting the spouse box
    // through `spouse_63f_boxes_count` (an ABSENT spouse whose boxes WOULD count forfeits too). r3 I-1:
    // this said "MFS never counts the spouse", which is no longer true on a qualifying MFS return.
    let taxpayer_no_blind = ri.header.taxpayer.blind.is_none();
    let spouse_blind_on_file = ri.header.spouse.as_ref().is_some_and(|s| s.blind.is_some());
    let spouse_no_blind =
        crate::tax::questions::spouse_63f_status_permits(ri) && !spouse_blind_on_file;
    let blind_persons = usize::from(taxpayer_no_blind) + usize::from(spouse_no_blind);
    if blind_persons > 0 {
        out.push(Advisory::BlindBoxForfeitedNotDeclared {
            per_box,
            persons: blind_persons,
        });
    }

    // ★ §2.7 / §3.4 — a declared MIXED-USE mortgage forgoes the interest v1 cannot allocate. The single
    // `mixed_use_mortgage_forgone` derivation decides liveness AND the ceiling (the same one that zeroed 8a
    // and checked the box); the text branches on the deduction the return actually took. Fires on the
    // ANSWERED "no" — a benefit forgone because the filer told us the truth is forgone just as hard.
    if let Some(forgone_interest) = mixed_use_mortgage_forgone(ri) {
        out.push(Advisory::MixedUseMortgageNotAllocated {
            forgone_interest,
            itemized: deduction_is_itemized,
        });
    }

    // ★ §164(b)(5) sales-tax election never asked (P9 §2.2, r5 Nit-3) — fires on `None` ∧ a Schedule A
    // exists, NOT "∧ the return itemizes": the unasked election can be exactly what would FLIP a
    // near-standard return into itemizing, and scoping by "itemizes" goes silent in that case.
    if ri
        .schedule_a
        .as_ref()
        .is_some_and(|a| a.salt_use_sales_tax.is_none())
    {
        out.push(Advisory::SalesTaxElectionNotAsked {
            itemized: deduction_is_itemized,
        });
    }

    // ★ Schedule C's Form-1099 pair. Fires on the SKIP only, and only where the form actually asks:
    // line I whenever there is a Schedule C, line J only once I is answered "Yes" (the form's own
    // "If 'Yes,'"). An answered box needs no advisory — re-nagging a filer who already decided is how
    // an advisory list teaches itself to be scrolled past.
    if let Some(c) = ri.schedule_c.as_ref() {
        let i_skipped = c.payments_requiring_1099.is_none();
        let j_skipped =
            c.payments_requiring_1099 == Some(true) && c.will_file_required_1099.is_none();
        if i_skipped || j_skipped {
            out.push(Advisory::ScheduleC1099NotAnswered {
                line_j_too: j_skipped,
            });
        }
    }

    // ★★ §G-22 — a §199A deduction claimed with no carryforward btctax can vouch for. Gated on
    // PROVENANCE, not merely on the value: a zero btctax wrote after computing last year is knowledge
    // and needs no note; a zero that is only the struct default is an unknown. Fires only when the
    // return actually claims the deduction, so a filer with no QBI never sees it.
    // ★ Gated on the INPUTS that mean "this return has §199A activity", not on the computed
    // deduction: the income limitation can zero the deduction while the carryforward is still wrong,
    // and a filer whose deduction was limited this year still carries the loss forward.
    let has_199a_activity =
        ri.schedule_c.is_some() || ri.div_1099.iter().any(|d| d.box5_section_199a > Usd::ZERO);
    if has_199a_activity {
        let unknown = |v: Usd, p: crate::tax::return_inputs::CarryProvenance| {
            v == Usd::ZERO && p == crate::tax::return_inputs::CarryProvenance::User
        };
        if unknown(
            ri.qbi.reit_ptp_carryforward_in,
            ri.qbi.reit_ptp_carryforward_in_provenance,
        ) || unknown(
            ri.qbi.qbi_carryforward_in,
            ri.qbi.qbi_carryforward_in_provenance,
        ) {
            out.push(Advisory::QbiCarryforwardNotStated);
        }
    }

    // ★★ §G-20 — the MFS spouse's §63(f) boxes, forgone because the i1040gi conditions are NOT met.
    // Fires only when the filer has actually TOLD us something that would have qualified the spouse,
    // so it never appears on an MFS return with no spouse data.
    //
    // ★★★ r3 I-1 — AND ONLY WHEN THE BOXES ARE NOT ACTUALLY CLAIMED. `fd9c15f` made them claimable
    // when all three conditions are affirmatively answered; this advisory predates that commit by two
    // and was left keyed on the filing status alone, so it fired on the very returns the fix enables,
    // telling a filer whose boxes btctax had ALREADY claimed that "the boxes are yours to check by
    // hand". Acting on it double-counts them — an UNDERSTATEMENT on a return signed under §6065.
    // The guard is now the same predicate the deduction asks.
    //
    // ★★★ PRE-MERGE finding 1 — AND ONLY WHEN ANSWERING COULD STILL RECOVER THE BOX. An adversely
    // ANSWERED condition disqualifies the spouse outright: btctax correctly declined the boxes, there
    // is nothing to recover, and telling the filer their tax is overstated invites them to claim a
    // deduction they are not entitled to. `spouse_not_filing_a_return == Some(false)` is the ORDINARY
    // MFS case — the spouse files their own return — so without this guard the advisory would be
    // wrong on the single commonest shape it can meet.
    let no_condition_is_adverse = ri.header.spouse_had_no_income != Some(false)
        && ri.header.spouse_not_filing_a_return != Some(false)
        && ri.header.can_be_claimed_as_dependent_spouse != Some(true);
    if ri.filing_status == FilingStatus::Mfs
        && !crate::tax::questions::spouse_63f_boxes_count(ri)
        && no_condition_is_adverse
    {
        if let Some(sp) = ri.header.spouse.as_ref() {
            let aged = sp
                .date_of_birth
                .is_some_and(|d| crate::tax::return_1040::born_early_enough(d, year));
            let blind = sp.blind == Some(true);
            let boxes = usize::from(aged) + usize::from(blind);
            if boxes > 0 {
                out.push(Advisory::Mfs63fSpouseBoxesForgone { per_box, boxes });
            }
        }
    }

    // ★★ §G-20a — the two BENEFIT carryovers, gated on provenance exactly like the QBI pair. Opposite
    // direction: omitting one costs the FILER, so §3.4 permits the omission — but only if they are told.
    let user = crate::tax::return_inputs::CarryProvenance::User;
    let cl = ri.capital_loss_carryforward_in.short == Usd::ZERO
        && ri.capital_loss_carryforward_in.long == Usd::ZERO
        && ri.capital_loss_carryforward_in_provenance == user;
    let ch = ri.charitable_carryover_in.is_empty() && ri.charitable_carryover_in_provenance == user;
    if cl || ch {
        out.push(Advisory::BenefitCarryoversNotStated {
            capital_loss: cl,
            charitable: ch,
        });
    }

    // FinCEN Notice 2020-2 — a declared foreign account.
    if ri.foreign_accounts == Some(true) {
        out.push(Advisory::FbarFinCen);
        // ★ …and its unnumbered sub-question skipped. Silence is lawful (nothing reads the box), but
        // the form prints a Caution beside it, so the skip is said out loud rather than passed over.
        if ri.fbar_filing_required.is_none() {
            out.push(Advisory::FbarSubQuestionNotAnswered);
        }
    }

    // The ledger's crypto donations assumed a public-charity donee.
    let donations = state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
        .count();
    if donations > 0 {
        out.push(Advisory::CharitableDoneeAssumedPublicCharity { donations });
    }

    // ★★★ §170(f)(11)(D) — over $500,000 CLAIMED for the property, the qualified appraisal is an
    //     ATTACHMENT to the return, not merely a record to keep.
    //
    // ★★ THE OPERAND IS `year_donation_deduction` — the pre-§170(b) claimed amount aggregated over
    //    all similar items — and NOT Schedule A line 12. Wiring it to the post-ceiling line would
    //    make a statutory attachment depend on AGI: the identical $700,000 gift would require an
    //    appraisal for a $3M-AGI filer and not for a $1M-AGI one, whose year-1 Schedule A allows only
    //    $300,000. Reg §1.170A-16(f)(3) settles it the other way by extending the duty to the
    //    carryover years, which only coheres if the trigger follows the CLAIM.
    //
    // ★ Strict `>` — §170(f)(11)(D) says "more than $500,000" (contrast §170(f)(8)'s "$250 or more").
    let claimed_for_property = crate::forms::year_donation_deduction(state, year);
    if claimed_for_property > crate::tax::tables::APPRAISAL_ATTACHMENT_THRESHOLD {
        out.push(Advisory::QualifiedAppraisalMustBeAttached {
            claimed: claimed_for_property,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{Dependent, ScheduleAInputs};
    use crate::tax::tables::SaltLimitation;

    /// The advisory set is driven by the inputs/ledger, and every message names the direction of the error
    /// (OVERSTATED) so a filer knows the omission costs them money, not the IRS.
    #[test]
    fn ctc_fires_on_dependents_and_says_overstated() {
        let a = Advisory::CtcOdcOmitted {
            dependents: 2,
            provably_zero: false,
        };
        let m = a.message();
        assert!(m.contains("2 dependent(s)"));
        assert!(m.contains("OVERSTATED"));
        assert!(m.contains("8812"));
    }

    #[test]
    fn dob_advisory_names_the_forfeited_amount() {
        let m = Advisory::AgedBoxForfeitedNoDob {
            per_box: dec!(1950),
        }
        .message();
        assert!(m.contains("$1,950"), "thousands-separated (P5-N5): {m}");
        assert!(m.contains("OVERSTATED"));
    }

    #[test]
    fn fbar_advisory_refuses_to_answer_for_you() {
        let m = Advisory::FbarFinCen.message();
        assert!(m.contains("never answers Schedule B Part III"));
    }

    #[test]
    fn donee_advisory_names_the_private_foundation_risk() {
        let m = Advisory::CharitableDoneeAssumedPublicCharity { donations: 1 }.message();
        assert!(m.contains("PRIVATE FOUNDATION"));
        assert!(m.contains("BASIS"));
    }

    /// A 2024 §170 Donation of one leg with the given holding-period `term`, `basis` and `fmv`.
    /// `claimed_deduction` is the §170(e) figure the fold computes — LT deducts FMV, ST deducts
    /// `min(FMV, basis)` — so a fixture cannot accidentally state a claim the legs do not support.
    fn donation_of(term: crate::state::Term, basis: Usd, fmv: Usd) -> LedgerState {
        use crate::event::BasisSource;
        use crate::identity::{EventId, LotId};
        use crate::state::{Removal, RemovalKind, RemovalLeg};
        let leg = RemovalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(1),
                split_sequence: 0,
            },
            sat: 100_000_000,
            basis,
            fmv_at_transfer: fmv,
            term,
            basis_source: BasisSource::ExchangeProvided,
            acquired_at: time::macros::date!(2020 - 01 - 01),
            pseudo: false,
        };
        let claimed = match term {
            crate::state::Term::LongTerm => fmv,
            crate::state::Term::ShortTerm => fmv.min(basis),
        };
        LedgerState {
            removals: vec![Removal {
                event: EventId::decision(1),
                kind: RemovalKind::Donation,
                removed_at: time::macros::date!(2024 - 06 - 01),
                legs: vec![leg],
                appraisal_required: false,
                donor_acquired_at: None,
                claimed_deduction: Some(claimed),
                donee: None,
            }],
            ..Default::default()
        }
    }

    /// ★★★ **P5 / §170(f)(11)(D) — THE ATTACH-THE-APPRAISAL GATE, on the adjudication's own two
    /// vectors.** Both must hold, and they pull in opposite directions, which is why one of them
    /// alone would be a green test that proves nothing.
    ///
    /// 1. **$700,000 gift, $1,000,000 AGI ⇒ the gate FIRES.** §170(b)'s 30% ceiling allows only
    ///    $300,000 on this year's Schedule A line 12, and that is IRRELEVANT: §170(f)(11)(D) keys on
    ///    the amount "claimed" for the property, which §170(f)(11)(F) and Reg §1.170A-16(f)(5)(ii)
    ///    make a property-level, similar-items aggregate — determined without regard to the §170(b)
    ///    ceiling or the §170(d) carryover split. Reg §1.170A-16(f)(3) confirms it by extending the
    ///    attach duty to the CARRYOVER years, which only coheres if the trigger follows the claim.
    ///    ★ The AGI-keyed reading is not merely different, it is absurd: the identical gift would
    ///    require an appraisal from a $3M-AGI filer and not from a $1M-AGI one.
    ///
    /// 2. **Short-term crypto, FMV $700,000, §170(e) basis-limited claim $180,000 ⇒ it does NOT
    ///    fire.** §170(e)(1)(A) reduces the deduction to basis for property whose sale would produce
    ///    ordinary or short-term gain, so the amount CLAIMED is $180,000 and no attachment is owed.
    ///    The operand is post-§170(e), pre-§170(b).
    ///
    /// **B1 mutations, each observed RED before the fix landed:**
    /// - cap the operand at the §170(b) 30%-of-AGI ceiling (the "wired to Schedule A line 12"
    ///   defect the adjudication names as the one to avoid) ⇒ vector 1 reds;
    /// - key the operand to raw contributed FMV instead of the §170(e) claim ⇒ vector 2 reds;
    /// - relax the threshold from `>` to `>=` ⇒ the exactly-$500,000 row reds.
    #[test]
    fn the_appraisal_attachment_gate_keys_on_the_pre_ceiling_claim_not_the_ceiling_or_the_fmv() {
        use crate::state::Term;
        let fired = |state: &LedgerState, agi: Usd| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            advisories(&ri, state, Usd::ZERO, agi, Usd::ZERO, &params(), 2024, true)
                .into_iter()
                .find_map(|a| match a {
                    Advisory::QualifiedAppraisalMustBeAttached { claimed } => Some(claimed),
                    _ => None,
                })
        };

        // VECTOR 1 — $700,000 long-term gift against $1,000,000 of AGI. The 30% ceiling allows
        // $300,000 this year; the gate fires on the $700,000 CLAIMED for the property regardless.
        assert_eq!(
            fired(
                &donation_of(Term::LongTerm, dec!(100000), dec!(700000)),
                dec!(1000000)
            ),
            Some(dec!(700000)),
            "the §170(b) AGI ceiling does not decide a §170(f)(11)(D) attachment — keying it to \
             Schedule A line 12 would make substantiation depend on AGI"
        );

        // …and the SAME gift at triple the AGI fires identically. If the operand were ceiling-limited
        // these two rows would disagree, which is the absurdity in one assertion.
        assert_eq!(
            fired(
                &donation_of(Term::LongTerm, dec!(100000), dec!(700000)),
                dec!(3000000)
            ),
            Some(dec!(700000)),
            "the identical property must owe the identical attachment at any AGI"
        );

        // VECTOR 2 — short-term crypto, FMV $700,000, basis $180,000. §170(e)(1)(A) limits the claim
        // to basis, so $180,000 is claimed and NO attachment is owed.
        assert_eq!(
            fired(
                &donation_of(Term::ShortTerm, dec!(180000), dec!(700000)),
                dec!(1000000)
            ),
            None,
            "the operand is the §170(e)-reduced CLAIM, not the fair market value contributed"
        );

        // THE BOUNDARY — §170(f)(11)(D) says "more than $500,000", so exactly $500,000 does not fire
        // (contrast §170(f)(8)'s "$250 or more", which does at exactly $250).
        assert_eq!(
            fired(
                &donation_of(Term::LongTerm, dec!(1), dec!(500000)),
                dec!(1000000)
            ),
            None,
            "exactly $500,000 is not MORE THAN $500,000"
        );
        assert_eq!(
            fired(
                &donation_of(Term::LongTerm, dec!(1), dec!(500000.01)),
                dec!(1000000)
            ),
            Some(dec!(500000.01)),
            "…and a cent over is"
        );
    }

    /// ★★ **The §170(f)(11)(D) advisory must distinguish itself from the $5,000 rule and name the
    /// carryover-year recurrence.** Both are "you need a qualified appraisal" to a skimming reader,
    /// and only one of them makes the appraisal part of the FILED return — a filer who reads this as
    /// "obtain and keep one" has done the wrong thing.
    ///
    /// B1 mutation: drop "ATTACH" or the Reg §1.170A-16(f)(3) sentence and the matching row reds.
    #[test]
    fn the_appraisal_attachment_advisory_says_attach_and_says_it_recurs() {
        let m = Advisory::QualifiedAppraisalMustBeAttached {
            claimed: dec!(700000),
        }
        .message();
        for phrase in [
            "ATTACH",
            "$500,000",
            "170(f)(11)(D)",
            "1.170A-16(f)(3)",
            "carryover",
            "$700,000", // the triggering amount, so the filer can check it against their own figure
        ] {
            assert!(m.contains(phrase), "message must say {phrase:?}: {m}");
        }
        // …and it must NOT be mistakable for the $5,000 obtain-and-keep rule.
        assert!(
            m.contains("$5,000"),
            "the message must say how this differs from the $5,000 rule: {m}"
        );
    }

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

    /// A high-income Single filer WITH a DOB, no dependents, no foreign account, no donations gets
    /// exactly the UNCONDITIONAL advisories — and no spurious ones.
    ///
    /// [★ P5-I2] This used to assert NO advisories. That was wrong, not merely untested: the
    /// residential-energy credit has no income limit at all, and the adoption credit reaches into the
    /// $250k band, so "v1 did not compute these" applies to every return ever produced. The test's
    /// real intent — the common case must not be NOISY — is preserved by pinning the exact set.
    ///
    /// ★★ **2026-07-31 (§G-20a): the unconditional set grew from ONE to TWO**, and the pinned list is
    /// the only thing that makes that visible. `BenefitCarryoversNotStated` fires on any return whose
    /// prior year btctax did not compute, which is every FIRST return — and that is correct rather
    /// than noisy: a carryover comes from a prior year, so no property of THIS year could narrow it,
    /// and gating on this year's activity would go silent exactly when it matters most (a filer with a
    /// large prior loss and no activity now). It goes quiet permanently once btctax computes a year.
    ///
    /// ★ **But two unconditional members is a UX budget worth watching** — the failure mode this
    /// codebase keeps citing is an advisory list that teaches itself to be scrolled past. Recorded in
    /// `FOLLOWUPS.md` §G-20a; if a third arrives, that is the moment to reconsider the surface rather
    /// than the individual advisory.
    #[test]
    fn a_clean_high_income_return_has_only_the_unconditional_omissions() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        ri.header.taxpayer.blind = Some(false); // a truly clean return has ANSWERED blindness, so no §63(f) blind note
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000), // earned
            dec!(150000), // AGI (1040 L11)
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert_eq!(
            got,
            vec![
                Advisory::OtherCreditsOmitted,
                // ★ §G-20a — the SECOND unconditional member; see this test's docs for why broad
                // firing is correct here and what would make it worth reconsidering.
                Advisory::BenefitCarryoversNotStated {
                    capital_loss: true,
                    charitable: true,
                },
            ],
            "{got:?}"
        );
    }

    /// Dependents fire the CTC omission; a missing DOB fires the §63(f) aged-box forfeit; low AGI with
    /// earned income fires the EIC omission — all three at once, in a stable order.
    #[test]
    fn omissions_fire_together_in_order() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.dependents = vec![Dependent::default(), Dependent::default()];
        // taxpayer.date_of_birth defaults to None → aged box forfeited. Blindness IS declared here so the
        // vector stays focused on the aged/CTC/EIC omissions this test is about (the blind note has its own).
        ri.header.taxpayer.blind = Some(false);
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(30000), // earned
            dec!(30000), // AGI (1040 L11)
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert_eq!(
            got,
            vec![
                Advisory::CtcOdcOmitted {
                    dependents: 2,
                    provably_zero: false
                },
                Advisory::EicOmitted,
                Advisory::OtherCreditsOmitted,
                Advisory::AgedBoxForfeitedNoDob {
                    per_box: dec!(1950)
                },
                Advisory::BenefitCarryoversNotStated {
                    capital_loss: true,
                    charitable: true,
                },
            ]
        );
    }

    /// The EIC advisory needs BOTH earned income and a low AGI — a low-AGI filer with no earned income
    /// (all investment income) does not get it.
    #[test]
    fn eic_needs_earned_income_and_low_agi() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        let p = params();
        // No earned income → no EIC advisory.
        assert!(!advisories(
            &ri,
            &LedgerState::default(),
            Usd::ZERO,
            dec!(30000), /* AGI (1040 L11) */
            Usd::ZERO,
            &p,
            2024,
            false
        )
        .contains(&Advisory::EicOmitted));
        // Earned income but AGI at/over the ceiling → no EIC advisory. [★ P5-I3] This leg used
        // $60,000, which is now BELOW the corrected $70,000 ceiling — the old fixture only passed
        // because the ceiling was too low. It must sit above the real one to be discriminating.
        assert!(!advisories(
            &ri,
            &LedgerState::default(),
            dec!(70000),
            dec!(70000), /* AGI (1040 L11) */
            Usd::ZERO,
            &p,
            2024,
            false
        )
        .contains(&Advisory::EicOmitted));
        // Earned income + low AGI → fires.
        assert!(advisories(
            &ri,
            &LedgerState::default(),
            dec!(30000),
            dec!(30000), /* AGI (1040 L11) */
            Usd::ZERO,
            &p,
            2024,
            false
        )
        .contains(&Advisory::EicOmitted));
    }

    /// A declared foreign account fires the FBAR disclosure; MFJ uses the married aged/blind rate.
    #[test]
    fn fbar_fires_and_mfj_uses_the_married_rate() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            foreign_accounts: Some(true),
            ..Default::default()
        };
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(200000),
            dec!(200000),
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert!(got.contains(&Advisory::FbarFinCen));
        assert!(got.contains(&Advisory::AgedBoxForfeitedNoDob {
            per_box: dec!(1550) // married rate
        }));
    }

    /// ★★ §G-20a — the BENEFIT carryovers, and the opposite direction from the QBI pair.
    ///
    /// Omitting these costs the FILER (they reduce tax), so §3.4 permits the omission — but only if
    /// they are told. Gated on `CarryProvenance` for the same reason as the QBI advisory: without it, a
    /// zero btctax COMPUTED last year is indistinguishable from one nobody ever stated, and the note
    /// nags a filer whose prior year btctax itself produced.
    #[test]
    fn the_benefit_carryover_advisory_needs_provenance_to_be_honest() {
        use crate::tax::return_inputs::CarryProvenance;
        let run = |cl: Usd, cl_p: CarryProvenance, ch_empty: bool, ch_p: CarryProvenance| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            ri.capital_loss_carryforward_in.long = cl;
            ri.capital_loss_carryforward_in_provenance = cl_p;
            if !ch_empty {
                ri.charitable_carryover_in = vec![crate::tax::return_inputs::CharitableCarryItem {
                    class: crate::tax::return_inputs::CharitableClass::Cash60,
                    amount: dec!(1000),
                    origin_year: 2023,
                    provenance: CarryProvenance::User,
                }];
            }
            ri.charitable_carryover_in_provenance = ch_p;
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .into_iter()
            .find_map(|a| match a {
                Advisory::BenefitCarryoversNotStated {
                    capital_loss,
                    charitable,
                } => Some((capital_loss, charitable)),
                _ => None,
            })
        };
        let u = CarryProvenance::User;
        let c = CarryProvenance::Computed;

        assert_eq!(
            run(Usd::ZERO, u, true, u),
            Some((true, true)),
            "both unknown ⇒ name both"
        );
        assert_eq!(
            run(Usd::ZERO, c, true, c),
            None,
            "★ btctax COMPUTED both zeros last year — that is knowledge, not an unknown"
        );
        assert_eq!(
            run(dec!(3000), u, true, u),
            Some((false, true)),
            "a stated capital-loss carryover says nothing about the charitable one"
        );
        assert_eq!(
            run(Usd::ZERO, u, false, u),
            Some((true, false)),
            "…and vice versa — an EMPTY list is the ambiguous case, a populated one is not"
        );

        // ★ The DIRECTION must be named, and it is the opposite of the QBI pair's.
        let m = Advisory::BenefitCarryoversNotStated {
            capital_loss: true,
            charitable: false,
        }
        .message();
        assert!(m.contains("costs YOU, not the Treasury"), "{m}");
        assert!(m.contains("OVERSTATED"), "{m}");
        assert!(
            Advisory::QbiCarryforwardNotStated
                .message()
                .contains("UNDERSTATED"),
            "the sibling goes the other way — if both ever say the same thing, one is wrong"
        );
    }
    /// ★★★ **PRE-MERGE finding 1 — r3 fixed the FIRING CONDITION and never touched the SENTENCE.**
    ///
    /// Three independent lenses found this and six skeptics failed to kill it. The message still said
    /// btctax *"counts a spouse's aged/blind boxes only on a JOINT return"* and called the three
    /// i1040gi conditions *"three things btctax does not ask and cannot verify"*. Both were falsified
    /// by `fd9c15f`: btctax asks all three, verifies all three, and claims the boxes when they are met.
    ///
    /// Two reachable shapes, and the second is the dangerous one:
    ///   (a) UNANSWERED — the default, because the input form deliberately exempts two of the three
    ///       conditions. Something IS recoverable, but the message named the wrong remedy: it told the
    ///       filer to check the boxes BY HAND, which drifts the §63(f) box count away from the line-12
    ///       amount `AgedBlindBoxes::count()` is the single source for. The remedy that works is to
    ///       ANSWER, whereupon the deduction and the boxes move together.
    ///   (b) ANSWERED ADVERSELY — the filer told btctax the spouse HAD income, btctax correctly
    ///       declined the boxes, and the advisory still offered a hand-claim on a §6065 return. That
    ///       is r3's own I-1 mechanism inverted, and it is fixed by NOT FIRING: nothing was forgone
    ///       that answering could recover.
    ///
    /// Mutation-verified: dropping the adverse-answer guard reds shape (b); reinstating either false
    /// sentence reds the prose row.
    #[test]
    fn the_mfs_63f_advisory_is_silent_when_a_condition_was_answered_adversely() {
        use crate::tax::return_inputs::Person;
        let old = time::macros::date!(1955 - 03 - 02);
        let run = |shape: &dyn Fn(&mut ReturnInputs)| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Mfs,
                ..Default::default()
            };
            ri.header.spouse = Some(Person {
                date_of_birth: Some(old),
                blind: Some(true),
                ..Default::default()
            });
            shape(&mut ri);
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .into_iter()
            .find(|a| matches!(a, Advisory::Mfs63fSpouseBoxesForgone { .. }))
        };

        // (a) NOTHING answered ⇒ the boxes are recoverable, so the filer must be told.
        let fired = run(&|_ri| {}).expect("an unanswered condition forgoes a recoverable box");

        // ★ …and the message must not carry either falsified sentence, nor send them to do it by hand.
        let msg = fired.message();
        for lie in [
            "only on a JOINT return",
            "does not ask and cannot verify",
            "yours to check by hand",
        ] {
            assert!(
                !msg.contains(lie),
                "the message still says {lie:?} — btctax asks, verifies, and claims these boxes now"
            );
        }
        assert!(
            msg.contains("income answer") || msg.contains("income import"),
            "it must name the action that actually works: {msg}"
        );

        // (b) ★ ANSWERED ADVERSELY ⇒ the spouse is disqualified, btctax correctly declined the boxes,
        //     and there is nothing to recover. Advising a hand-claim here invites an UNDERSTATEMENT.
        assert!(
            run(&|ri| ri.header.spouse_had_no_income = Some(false)).is_none(),
            "the filer said the spouse HAD income — the boxes are correctly declined, say nothing"
        );
        assert!(
            run(&|ri| ri.header.spouse_not_filing_a_return = Some(false)).is_none(),
            "the spouse files their own return — the ORDINARY MFS case, and it disqualifies them"
        );
        assert!(
            run(&|ri| ri.header.can_be_claimed_as_dependent_spouse = Some(true)).is_none(),
            "claimable as someone's dependent — disqualified (this also refuses upstream)"
        );

        // (c) A partially-answered set still fires: one adverse answer disqualifies, but two claiming
        //     answers plus one SILENCE is still recoverable.
        assert!(
            run(&|ri| {
                ri.header.spouse_had_no_income = Some(true);
                ri.header.spouse_not_filing_a_return = Some(true);
            })
            .is_some(),
            "two of three answered in the claiming direction — the third is still worth asking for"
        );
    }

    /// ★★★ **r3 I-1 — the review's top finding, and an UNDERSTATEMENT path.** `fd9c15f` made the MFS
    /// spouse's §63(f) boxes CLAIMABLE when all three i1040gi conditions are affirmatively met, and
    /// routed the deduction through `questions::spouse_63f_boxes_count`. The four §63(f) advisories,
    /// written two commits earlier, were left keyed on `filing_status == Mfj` — so on exactly the
    /// returns the fix exists to enable, `Mfs63fSpouseBoxesForgone` still fired, telling the filer
    /// their tax was OVERSTATED and *"the boxes are yours to check by hand"* while btctax had already
    /// claimed them. A filer who acted on it would DOUBLE-COUNT the boxes on a return signed
    /// under §6065.
    ///
    /// The advisory now asks the same predicate the deduction asks. Both directions are pinned: it
    /// must be SILENT when the boxes are claimed, and must still FIRE when they are genuinely forgone.
    ///
    /// Mutation-verified: reverting the guard to `filing_status == Mfs` alone reds the first block.
    #[test]
    fn the_mfs_63f_advisory_is_silent_once_the_boxes_are_actually_claimed() {
        let old = time::macros::date!(1955 - 03 - 02); // 65+ in 2024
        let run = |qualify: bool| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Mfs,
                ..Default::default()
            };
            ri.header.spouse = Some(crate::tax::return_inputs::Person {
                date_of_birth: Some(old),
                blind: Some(true),
                ..Default::default()
            });
            if qualify {
                // The three i1040gi conditions, in the CLAIMING direction.
                ri.header.spouse_had_no_income = Some(true);
                ri.header.spouse_not_filing_a_return = Some(true);
                ri.header.can_be_claimed_as_dependent_spouse = Some(false);
            }
            let counted = crate::tax::questions::spouse_63f_boxes_count(&ri);
            let fired = advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .into_iter()
            .any(|a| matches!(a, Advisory::Mfs63fSpouseBoxesForgone { .. }));
            (counted, fired)
        };

        // ★ QUALIFYING: the boxes ARE counted, so nothing was forgone and the advisory must be SILENT.
        assert_eq!(
            run(true),
            (true, false),
            "the boxes are claimed — an advisory saying they were forgone invites the filer to \
             claim them a SECOND time"
        );

        // ★ NOT QUALIFYING: the boxes are genuinely forgone, so the advisory must still fire. This is
        //   the half a careless fix would break, leaving a silent conservative omission (§3.4).
        assert_eq!(
            run(false),
            (false, true),
            "unanswered conditions forgo the boxes — §3.4 permits that only if the filer is TOLD"
        );
    }

    /// ★★ **r3 I-1, the FALSE-NEGATIVE half.** The three *forfeit* advisories counted the spouse only
    /// on MFJ, so on a QUALIFYING MFS return — where the boxes are now claimable — a spouse with no
    /// DOB, an unanswered death question, or an undeclared blindness forfeited a box in silence.
    /// Each one now uses `spouse_63f_boxes_count`, i.e. it speaks exactly when a spouse box was on
    /// the table to lose.
    ///
    /// Mutation-verified: reverting any one guard to `== Mfj` reds its own row by name.
    #[test]
    fn the_forfeit_advisories_speak_for_a_qualifying_mfs_spouse() {
        use crate::tax::return_inputs::Person;
        let old = time::macros::date!(1955 - 03 - 02);
        let qualifying_mfs = |sp: Person| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Mfs,
                ..Default::default()
            };
            ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1990 - 01 - 01));
            ri.header.taxpayer.blind = Some(false);
            ri.header.spouse = Some(sp);
            ri.header.spouse_had_no_income = Some(true);
            ri.header.spouse_not_filing_a_return = Some(true);
            ri.header.can_be_claimed_as_dependent_spouse = Some(false);
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
        };
        let has = |v: &[Advisory], f: fn(&Advisory) -> bool| v.iter().any(f);

        // (1) No DOB on file ⇒ the aged box cannot be evaluated at all.
        let v = qualifying_mfs(Person {
            date_of_birth: None,
            blind: Some(false),
            ..Default::default()
        });
        assert!(
            has(&v, |a| matches!(a, Advisory::AgedBoxForfeitedNoDob { .. })),
            "a qualifying MFS spouse with no DOB forfeits the aged box — say so"
        );

        // (2) A qualifying DOB but the death question was never answered ⇒ `is_aged` forgoes.
        let v = qualifying_mfs(Person {
            date_of_birth: Some(old),
            blind: Some(false),
            ..Default::default()
        });
        assert!(
            has(&v, |a| matches!(
                a,
                Advisory::AgedBoxForfeitedDeathUnanswered { .. }
            )),
            "an unanswered death question forgoes the aged box on a qualifying MFS return too"
        );

        // (3) Blindness never declared ⇒ the blind box is forgone.
        let v = qualifying_mfs(Person {
            date_of_birth: Some(old),
            blind: None,
            ..Default::default()
        });
        assert!(
            has(&v, |a| matches!(
                a,
                Advisory::BlindBoxForfeitedNotDeclared { .. }
            )),
            "an undeclared blindness forgoes the box on a qualifying MFS return too"
        );
    }

    /// ★★ §G-20 — an MFS return forgoes the spouse's §63(f) boxes, and until now said nothing.
    ///
    /// The forfeit is lawful (it OVERSTATES tax; btctax cannot verify the three conditions i1040gi
    /// requires) but §3.4 permits a conservative omission **only if the filer is told**.
    ///
    /// ★ The sharp part: btctax **asks** an MFS filer whether their spouse is blind — `BlindSpouse` is
    /// live on `spouse.is_some()`, not on MFJ — and then discards the answer. So it fires exactly when
    /// the filer has told us something that would have counted.
    #[test]
    fn the_mfs_spouse_63f_advisory_fires_only_when_a_box_was_actually_forgone() {
        let run = |status: FilingStatus, dob: Option<time::Date>, blind: Option<bool>| {
            let mut ri = ReturnInputs {
                filing_status: status,
                ..Default::default()
            };
            ri.header.spouse = Some(crate::tax::return_inputs::Person {
                date_of_birth: dob,
                blind,
                ..Default::default()
            });
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .into_iter()
            .find_map(|a| match a {
                Advisory::Mfs63fSpouseBoxesForgone { boxes, .. } => Some(boxes),
                _ => None,
            })
        };
        let old = Some(time::macros::date!(1955 - 03 - 02)); // 65+ in 2024
        let young = Some(time::macros::date!(1990 - 01 - 01));

        assert_eq!(
            run(FilingStatus::Mfs, old, Some(true)),
            Some(2),
            "★ aged AND blind ⇒ two boxes forgone"
        );
        assert_eq!(run(FilingStatus::Mfs, old, None), Some(1), "aged only");
        assert_eq!(
            run(FilingStatus::Mfs, young, Some(true)),
            Some(1),
            "blind only — and btctax ASKED for this on MFS, then discarded it"
        );
        assert_eq!(
            run(FilingStatus::Mfs, young, Some(false)),
            None,
            "nothing qualifies ⇒ nothing forgone ⇒ silence"
        );
        assert_eq!(
            run(FilingStatus::Mfs, None, None),
            None,
            "★ no spouse data ⇒ nothing was told to us, so there is nothing to have discarded"
        );
        assert_eq!(
            run(FilingStatus::Mfj, old, Some(true)),
            None,
            "★★ MFJ COUNTS the boxes — an advisory there would be flatly false"
        );

        // The message must name the whole forfeit, not one box.
        let m = Advisory::Mfs63fSpouseBoxesForgone {
            per_box: dec!(1550),
            boxes: 2,
        }
        .message();
        assert!(m.contains("$3,100 in total"), "{m}");
        assert!(m.contains("OVERSTATED"), "the direction must be named: {m}");
    }

    /// ★★★ §G-22 — the two QBI loss carryforwards were IMPORT-ONLY, and they are the only carryforward
    /// family whose omission UNDERSTATES tax.
    ///
    /// Gated on PROVENANCE, not on the value. That is the whole design: a zero btctax wrote itself
    /// after computing last year is KNOWLEDGE (`Computed`) and needs no note; a zero that is merely the
    /// struct default is an UNKNOWN (`User`) and does. Without that distinction the advisory would
    /// either nag forever or go silent exactly when it matters.
    #[test]
    fn the_qbi_carryforward_advisory_distinguishes_a_known_zero_from_an_unknown_one() {
        use crate::tax::return_inputs::CarryProvenance;
        let run = |sched_c: bool, reit: Usd, prov: CarryProvenance| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            if sched_c {
                ri.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
                    business_description: "Bitcoin mining".into(),
                    ..Default::default()
                });
            }
            // ★ BOTH leaves, not one. Each carryforward is independently unknown, so a fixture that
            // states only the REIT one leaves the business one an unknown zero — and the advisory
            // correctly still fires. That is the behaviour, and stating only one hid it.
            ri.qbi.reit_ptp_carryforward_in = reit;
            ri.qbi.qbi_carryforward_in = reit;
            ri.qbi.reit_ptp_carryforward_in_provenance = prov;
            ri.qbi.qbi_carryforward_in_provenance = prov;
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .contains(&Advisory::QbiCarryforwardNotStated)
        };

        assert!(
            run(true, Usd::ZERO, CarryProvenance::User),
            "★ §199A activity + a zero btctax was never told about ⇒ say so. Omitting a prior-year \
             QBI loss INFLATES the deduction and UNDERSTATES the tax."
        );
        assert!(
            !run(true, Usd::ZERO, CarryProvenance::Computed),
            "★ a zero btctax COMPUTED last year is knowledge, not an unknown — silence is right"
        );
        assert!(
            !run(true, dec!(20000), CarryProvenance::User),
            "the filer stated a figure ⇒ nothing unknown"
        );
        // ★ EITHER unknown is enough — they are independent lines (8995 line 7 and line 3), and a
        // filer who states one has said nothing about the other.
        {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            ri.schedule_c = Some(crate::tax::return_inputs::ScheduleCInputs {
                business_description: "Bitcoin mining".into(),
                ..Default::default()
            });
            ri.qbi.reit_ptp_carryforward_in = dec!(20000); // stated
            ri.qbi.reit_ptp_carryforward_in_provenance = CarryProvenance::User;
            ri.qbi.qbi_carryforward_in = Usd::ZERO; // NOT stated
            ri.qbi.qbi_carryforward_in_provenance = CarryProvenance::User;
            assert!(
                advisories(
                    &ri,
                    &LedgerState::default(),
                    dec!(90000),
                    dec!(90000),
                    Usd::ZERO,
                    &params(),
                    2024,
                    false,
                )
                .contains(&Advisory::QbiCarryforwardNotStated),
                "one carryforward stated says nothing about the other"
            );
        }
        assert!(
            !run(false, Usd::ZERO, CarryProvenance::User),
            "★ NO §199A activity ⇒ the deduction does not arise, so there is nothing to get wrong"
        );

        let m = Advisory::QbiCarryforwardNotStated.message();
        assert!(
            m.contains("UNDERSTATED"),
            "the direction must be named: {m}"
        );
        assert!(
            m.contains("lines 16 and 17"),
            "…and where to find the figure: {m}"
        );
    }

    /// ★★ Schedule C's Form-1099 pair advises on the SKIP only, and distinguishes the two places a
    /// filer can stop. Stopping after a **Yes** on line I is strictly worse — they have already
    /// declared the payments exist — so the message says so and the flag carries it.
    ///
    /// ★ It deliberately does NOT fire when there is no Schedule C: the form does not ask, so there is
    /// nothing skipped. An advisory that fires where the question was never posed is noise, and noise
    /// is how an advisory list teaches itself to be scrolled past.
    #[test]
    fn the_schedule_c_1099_pair_advises_only_on_the_skip() {
        let run = |sc: Option<(Option<bool>, Option<bool>)>| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            ri.schedule_c = sc.map(|(i, j)| crate::tax::return_inputs::ScheduleCInputs {
                business_description: "Bitcoin mining".into(),
                payments_requiring_1099: i,
                will_file_required_1099: j,
                ..Default::default()
            });
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .into_iter()
            .find_map(|a| match a {
                Advisory::ScheduleC1099NotAnswered { line_j_too } => Some(line_j_too),
                _ => None,
            })
        };

        assert_eq!(
            run(None),
            None,
            "no Schedule C ⇒ the form never asks ⇒ silence"
        );
        assert_eq!(
            run(Some((None, None))),
            Some(false),
            "line I skipped ⇒ advise, and line J was never reached"
        );
        assert_eq!(
            run(Some((Some(true), None))),
            Some(true),
            "★ answered I=Yes then stopped ⇒ the WORSE skip: the payments are already declared"
        );
        assert_eq!(
            run(Some((Some(true), Some(true)))),
            None,
            "both answered ⇒ nothing forgone, nothing to say"
        );
        assert_eq!(
            run(Some((Some(true), Some(false)))),
            None,
            "answered I=Yes, J=No — an ANSWER, not a skip. btctax does not editorialise on it."
        );
        assert_eq!(
            run(Some((Some(false), None))),
            None,
            "I=No ⇒ the form does not ask J, so nothing is skipped"
        );

        // ★ The message names BOTH sections — the exposure is the point of the advisory.
        let m = Advisory::ScheduleC1099NotAnswered { line_j_too: false }.message();
        assert!(m.contains("§6721") && m.contains("§6722"), "{m}");
    }

    /// ★★ Schedule B 7a's FBAR SUB-QUESTION is class (B): skipping it is lawful, so it must not
    /// refuse — but it must not be silent either. The advisory fires **exactly** on the skip.
    ///
    /// The three cases are the whole contract, and the middle one is why the advisory is conditioned
    /// on `is_none()` rather than on 7a: a filer who ANSWERED (either way) has already made the
    /// decision, and re-nagging them would train the advisory list to be ignored.
    #[test]
    fn the_fbar_sub_question_advises_only_when_it_was_skipped() {
        let run = |fbar: Option<bool>| {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                foreign_accounts: Some(true),
                fbar_filing_required: fbar,
                ..Default::default()
            };
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(200000),
                dec!(200000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
        };
        assert!(
            run(None).contains(&Advisory::FbarSubQuestionNotAnswered),
            "skipped ⇒ the box prints blank, so say so"
        );
        for answered in [Some(true), Some(false)] {
            assert!(
                !run(answered).contains(&Advisory::FbarSubQuestionNotAnswered),
                "answered {answered:?} ⇒ the box is filled; no advisory"
            );
        }
        // Never fires without a 7a "Yes" — the FORM does not ask the sub-question then.
        for seven_a in [None, Some(false)] {
            let ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                foreign_accounts: seven_a,
                ..Default::default()
            };
            assert!(!advisories(
                &ri,
                &LedgerState::default(),
                dec!(200000),
                dec!(200000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .contains(&Advisory::FbarSubQuestionNotAnswered));
        }
        // ★ The message quotes Schedule B's own Caution VERBATIM — the words the filer can find on
        // their paperwork, not a paraphrase of them.
        assert!(Advisory::FbarSubQuestionNotAnswered.message().contains(
            "If required, failure to file FinCEN Form 114 may result in substantial penalties."
        ));
    }

    /// ★★ The §G-9 death gate became SKIPPABLE, so the age-65 box can now be forgone by silence — and
    /// the filer must be told, but ONLY when the silence actually costs them something.
    ///
    /// The precision is the point. Firing whenever the gate is unanswered would put this advisory on
    /// nearly every return (it is always live and most filers skip it), which trains the whole advisory
    /// list to be scrolled past. It fires iff a date of birth on file WOULD have qualified them.
    #[test]
    fn the_forgone_aged_box_advises_only_when_the_skip_actually_costs_something() {
        let run = |dob: Option<time::Date>, died: Option<bool>| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            };
            ri.header.taxpayer.date_of_birth = dob;
            ri.header.taxpayer_died_during_year = died;
            advisories(
                &ri,
                &LedgerState::default(),
                dec!(90000),
                dec!(90000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
            .iter()
            .any(|a| matches!(a, Advisory::AgedBoxForfeitedDeathUnanswered { .. }))
        };
        let qualifying = Some(time::macros::date!(1955 - 03 - 02)); // 65+ in 2024
        let too_young = Some(time::macros::date!(1990 - 01 - 01));

        assert!(
            run(qualifying, None),
            "a qualifying DOB + an unanswered gate ⇒ the box is forgone; say so"
        );
        assert!(
            !run(qualifying, Some(false)),
            "answered ⇒ the box is CLAIMED, nothing forgone"
        );
        assert!(
            !run(qualifying, Some(true)),
            "answered \"died\" ⇒ the DATE skippable governs, not this advisory"
        );
        assert!(
            !run(too_young, None),
            "★ too young ⇒ the skip costs NOTHING; an advisory here would be noise on nearly every return"
        );
        assert!(
            !run(None, None),
            "no DOB ⇒ `AgedBoxForfeitedNoDob` already covers it; two advisories for one forfeit is worse than one"
        );
        // ★ The message must quote the WHOLE forfeit, not one box. An MFJ couple with two boxes
        // forgone was told $1,550 when $3,100 was at stake — a wrong number inside the sentence whose
        // job is to make the number vivid. `persons` was computed and used for the pronoun only.
        let one = Advisory::AgedBoxForfeitedDeathUnanswered {
            per_box: dec!(1550),
            persons: 1,
        }
        .message();
        assert!(one.contains("worth $1,550"), "{one}");
        let two = Advisory::AgedBoxForfeitedDeathUnanswered {
            per_box: dec!(1550),
            persons: 2,
        }
        .message();
        assert!(
            two.contains("worth $3,100"),
            "two boxes forgone ⇒ the value of answering is 2 × per-box: {two}"
        );
        assert!(
            two.contains("($1,550 per box)"),
            "…and per-box stays per-box: {two}"
        );
    }

    /// ★ **P5-I3 regression — the exact household the reviewer reproduced.** MFJ, 3 dependents,
    /// $63,000 of earned AGI: squarely inside EIC territory (the TY2024 MFJ 3-child AGI limit is
    /// $66,819; max credit $7,830), yet the old $60,000 ceiling fired NO EIC advisory. The ceiling
    /// had been set from the *Single/HoH/QSS* 3-child figure ($59,899), so every MFJ band above it
    /// was silently missed — an under-fire, which is the one direction §3.4 promises never to fail in.
    #[test]
    fn eic_advisory_fires_for_mfj_at_63k_p5_i3() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        ri.header.dependents = vec![
            Dependent::default(),
            Dependent::default(),
            Dependent::default(),
        ];
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(63000), // earned
            dec!(63000), // AGI — under the real MFJ 3-child limit of $66,819
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert!(
            got.contains(&Advisory::EicOmitted),
            "MFJ/$63k/3 kids must fire the EIC omission: {got:?}"
        );
        // The old $60,000 ceiling is the mutation: it would NOT have fired here.
        assert!(dec!(63000) > dec!(60000) && dec!(63000) < EIC_ADVISORY_AGI_CEILING);
    }

    /// ★ **P5-I2** — the two OMISSIONS rows that LIMITATIONS.md promised fire an advisory, and which
    /// previously fired nothing at all. `OtherCreditsOmitted` is unconditional (v1 captures no input
    /// that could establish eligibility for any of those credits); `RefundByPaperCheck` fires only
    /// when the return is actually due a refund, since it is not actionable otherwise.
    #[test]
    fn other_credits_and_paper_check_advisories_fire_p5_i2() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        let p = params();

        // Owes → other-credits fires, paper-check does NOT (nothing to deposit).
        let owes = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000),
            dec!(150000), /* AGI (1040 L11) */
            Usd::ZERO,    // no refund
            &p,
            2024,
            false,
        );
        assert!(owes.contains(&Advisory::OtherCreditsOmitted));
        assert!(!owes
            .iter()
            .any(|a| matches!(a, Advisory::RefundByPaperCheck { .. })));

        // Due a refund → both fire, and the message names the amount and the check.
        let refunded = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000),
            dec!(150000), /* AGI (1040 L11) */
            dec!(1234.56),
            &p,
            2024,
            false,
        );
        assert!(refunded.contains(&Advisory::OtherCreditsOmitted));
        assert!(refunded.contains(&Advisory::RefundByPaperCheck {
            refund: dec!(1234.56)
        }));
        let m = Advisory::RefundByPaperCheck {
            refund: dec!(1234.56),
        }
        .message();
        assert!(m.contains("$1,234.56"), "{m}");
        assert!(m.contains("mail a check"), "{m}");

        // The other-credits message must name the forms a filer would need to go claim.
        let oc = Advisory::OtherCreditsOmitted.message();
        for form in ["8863", "2441", "8880", "5695", "8839"] {
            assert!(
                oc.contains(form),
                "other-credits advisory must name Form {form}: {oc}"
            );
        }
        assert!(oc.contains("OVERSTATED"));
    }

    /// ★ **P5-M2** — on MFJ an ABSENT spouse record forfeits the spouse's §63(f) aged box exactly as
    /// a spouse record with a missing DOB does (`standard_deduction` counts spouse boxes only when
    /// the record exists), and nothing requires the record. It used to forfeit it SILENTLY.
    #[test]
    fn mfj_with_no_spouse_record_still_advises_the_aged_box_p5_m2() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        // The taxpayer HAS a DOB, so only the absent spouse can trigger this.
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        assert!(ri.header.spouse.is_none());

        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(200000),
            dec!(200000),
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert!(
            got.contains(&Advisory::AgedBoxForfeitedNoDob {
                per_box: dec!(1550) // married rate
            }),
            "an absent MFJ spouse record must not forfeit the aged box silently: {got:?}"
        );
    }

    /// A Single filer WITH a DOB (so the aged-box advisory is quiet) and a Schedule A reporting mortgage
    /// interest, with the mixed-use answer supplied by the caller.
    fn mixed_use_ri(answer: Option<bool>, interest: Usd) -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        ri.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: interest,
            mortgage_all_used_to_buy_build_improve: answer,
            ..Default::default()
        });
        ri
    }

    /// ★ P9 §2.7 / §3.4 — a declared MIXED-USE mortgage (`Some(false)`) forgoes the mortgage-interest
    /// deduction v1 cannot allocate, so the owner mandate fires a loud note. It fires on the ANSWERED "no"
    /// (never on `None`, which refuses upstream), carries the FULL 1098 interest as the ceiling, and records
    /// which deduction the return took so the text can be truthful about the form.
    #[test]
    fn mixed_use_mortgage_advisory_fires_on_declared_no() {
        let ri = mixed_use_ri(Some(false), dec!(20000));
        // Itemizing return.
        let itemized = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000),
            dec!(150000),
            Usd::ZERO,
            &params(),
            2024,
            true,
        );
        assert!(itemized.contains(&Advisory::MixedUseMortgageNotAllocated {
            forgone_interest: dec!(20000),
            itemized: true,
        }));
        // Standard-wins return — same forgone ceiling, but the flag records the standard deduction.
        let standard = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000),
            dec!(150000),
            Usd::ZERO,
            &params(),
            2024,
            false,
        );
        assert!(standard.contains(&Advisory::MixedUseMortgageNotAllocated {
            forgone_interest: dec!(20000),
            itemized: false,
        }));
    }

    /// The advisory is silent unless the filer DECLARED a mixed-use mortgage on a Schedule A with interest:
    /// `Some(true)` (all acquisition debt), `None` (unanswered — refuses upstream), $0 interest, and no
    /// Schedule A at all each fire nothing.
    #[test]
    fn mixed_use_mortgage_advisory_quiet_unless_declared_no() {
        let quiet = |ri: &ReturnInputs| {
            !advisories(
                ri,
                &LedgerState::default(),
                dec!(150000),
                dec!(150000),
                Usd::ZERO,
                &params(),
                2024,
                true,
            )
            .iter()
            .any(|a| matches!(a, Advisory::MixedUseMortgageNotAllocated { .. }))
        };
        assert!(
            quiet(&mixed_use_ri(Some(true), dec!(20000))),
            "acquisition-only"
        );
        assert!(quiet(&mixed_use_ri(None, dec!(20000))), "unanswered");
        assert!(
            quiet(&mixed_use_ri(Some(false), Usd::ZERO)),
            "no interest ⇒ not live"
        );
        let no_sched_a = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        assert!(quiet(&no_sched_a), "no Schedule A");
    }

    /// ★ P9 §2.2/§3.4 — §63(f) BLINDNESS forgone. Fires on `None` (never asked), never on `Some(false)`;
    /// counts the spouse box only on MFJ (mirrors `AgedBlindBoxes::for_return` — MFS never counts the
    /// spouse, and an ABSENT MFJ spouse still forfeits). Same statute/dollars as the aged box; they STACK.
    #[test]
    fn blind_advisory_counts_taxpayer_and_mfj_spouse_and_fires_on_none() {
        let dob = time::macros::date!(1980 - 01 - 01); // suppresses the aged advisory
        let go = |ri: &ReturnInputs| {
            advisories(
                ri,
                &LedgerState::default(),
                dec!(150000),
                dec!(150000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
        };
        let has_blind = |ri: &ReturnInputs, per_box, persons| {
            go(ri).contains(&Advisory::BlindBoxForfeitedNotDeclared { per_box, persons })
        };

        // Single, blindness unasked → persons = 1, unmarried rate.
        let mut single = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        single.header.taxpayer.date_of_birth = Some(dob);
        assert!(has_blind(&single, dec!(1950), 1));

        // Declared NOT blind → the advisory is silent (fires on None, not Some(false)).
        let mut declared = single.clone();
        declared.header.taxpayer.blind = Some(false);
        assert!(
            !go(&declared)
                .iter()
                .any(|a| matches!(a, Advisory::BlindBoxForfeitedNotDeclared { .. })),
            "declared-not-blind must be silent"
        );

        // MFJ, taxpayer blindness unasked + spouse ABSENT → persons = 2, married rate.
        let mut mfj = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        mfj.header.taxpayer.date_of_birth = Some(dob);
        assert!(
            has_blind(&mfj, dec!(1550), 2),
            "MFJ absent spouse forfeits too"
        );

        // MFS never counts the spouse box, even with a spouse Person present → persons = 1.
        let mut mfs = ReturnInputs {
            filing_status: FilingStatus::Mfs,
            ..Default::default()
        };
        mfs.header.taxpayer.date_of_birth = Some(dob);
        mfs.header.spouse = Some(Default::default());
        assert!(
            has_blind(&mfs, dec!(1550), 1),
            "MFS: spouse box is not this filer's"
        );
    }

    /// ★ §2.2/§3.4 (r5 Nit-3) — the §164(b)(5) sales-tax election was never asked. Fires on `None` ∧ a
    /// Schedule A EXISTS — NOT "∧ the return itemizes", which would go silent exactly when the unasked
    /// election is what would flip the return into itemizing.
    #[test]
    fn sales_tax_election_advisory_fires_on_none_with_a_schedule_a() {
        // Acquisition-only mortgage (no mixed-use advisory), Schedule A present, SALT election unasked.
        let mut ri = mixed_use_ri(Some(true), dec!(1));
        let go = |ri: &ReturnInputs| {
            advisories(
                ri,
                &LedgerState::default(),
                dec!(150000),
                dec!(150000),
                Usd::ZERO,
                &params(),
                2024,
                false,
            )
        };
        assert!(go(&ri).contains(&Advisory::SalesTaxElectionNotAsked { itemized: false }));

        // Answered → silent.
        ri.schedule_a.as_mut().unwrap().salt_use_sales_tax = Some(false);
        assert!(!go(&ri).contains(&Advisory::SalesTaxElectionNotAsked { itemized: false }));

        // No Schedule A → not live.
        let no_a = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        assert!(!go(&no_a).contains(&Advisory::SalesTaxElectionNotAsked { itemized: false }));
    }

    /// ★ §3.4 (r5 M-1) — the message TEXT branches on the deduction actually taken: the itemized branch
    /// names the Schedule A the filer filed; the standard branch must NOT describe a form they did not file.
    /// Both name the ceiling as "up to {forgone}", never as "the amount you lost".
    #[test]
    fn mixed_use_mortgage_advisory_text_branches_on_deduction_taken() {
        let itemized = Advisory::MixedUseMortgageNotAllocated {
            forgone_interest: dec!(20000),
            itemized: true,
        }
        .message();
        assert!(itemized.contains("Schedule A"), "{itemized}");
        assert!(itemized.contains("line 8a"), "{itemized}");
        assert!(
            itemized.contains("up to $20,000"),
            "the ceiling, comma-grouped: {itemized}"
        );
        assert!(itemized.contains("OVERSTATED"), "{itemized}");

        let standard = Advisory::MixedUseMortgageNotAllocated {
            forgone_interest: dec!(20000),
            itemized: false,
        }
        .message();
        assert!(standard.contains("standard deduction"), "{standard}");
        assert!(standard.contains("up to $20,000"), "{standard}");
        // ★ It must not tell a standard-deduction filer their Schedule A claimed anything (r5 M-1).
        assert!(
            !standard.contains("Your Schedule A claimed"),
            "the standard branch must not describe a form the filer did not file: {standard}"
        );
    }

    /// The two new class-(B) advisories name the direction of the error (OVERSTATED) and, for the blind box,
    /// the forfeited per-box amount (thousands-separated, like every printed figure).
    #[test]
    fn blind_and_sales_tax_advisories_name_the_stakes() {
        let blind = Advisory::BlindBoxForfeitedNotDeclared {
            per_box: dec!(1950),
            persons: 1,
        }
        .message();
        assert!(blind.contains("$1,950"), "thousands-separated: {blind}");
        assert!(blind.contains("§63(f)"), "{blind}");
        assert!(blind.contains("OVERSTATED"), "{blind}");

        let salt = Advisory::SalesTaxElectionNotAsked { itemized: true }.message();
        assert!(
            salt.contains("§164(b)(5)") || salt.contains("sales tax"),
            "{salt}"
        );
        assert!(salt.contains("OVERSTATED"), "{salt}");
        assert!(salt.contains("income answer"), "names the exit: {salt}");
    }

    /// ★ r3 MINOR-3 — the SALT advisory text branches on the deduction taken: the itemized filer's Schedule
    /// A "used" income taxes, but the standard filer filed none, so the text must not say so (the r5 M-1
    /// shape the sibling mixed-use advisory already honors).
    #[test]
    fn sales_tax_advisory_does_not_describe_a_form_the_standard_filer_did_not_file() {
        let itemized = Advisory::SalesTaxElectionNotAsked { itemized: true }.message();
        assert!(
            itemized.contains("your Schedule A used"),
            "the itemized filer DID file a Schedule A: {itemized}"
        );
        let standard = Advisory::SalesTaxElectionNotAsked { itemized: false }.message();
        assert!(
            !standard.contains("your Schedule A used"),
            "the standard filer filed NO Schedule A — do not say it 'used' income taxes: {standard}"
        );
        assert!(standard.contains("standard deduction"), "{standard}");
        assert!(standard.contains("OVERSTATED"), "{standard}");
    }
}
