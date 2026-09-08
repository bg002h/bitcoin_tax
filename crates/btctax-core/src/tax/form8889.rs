//! **Form 8889 — Health Savings Accounts (HSAs)**, transcribed line by line (T16 / FR-76).
//!
//! Every field below is one numbered line of the form, named for that line, carrying the official
//! printed text **verbatim** as its doc comment (`CLAUDE.md`, *"Transcribe IRS forms — never
//! paraphrase them"*). The words come from the TEXT LAYER — `design/forms/extract/f8889--2024.txt`
//! for the form, `i8889--2024.txt` for the instructions — never from a rendered page.
//!
//! ★★ **One struct serves TY2024 and TY2025, and that is measured rather than assumed.** Diffing the
//! two extracts with the year and the §223(b) figures normalised leaves exactly two differences: a
//! period after *"(see instructions)"* on line 13, and the 2025 footer's `Created 3/28/25`. The LINE
//! SET is identical, so a per-revision fork would be a fork with no line in it. The figures that DO
//! move are [`crate::tax::tables::HsaParams`], read from the year's own params.
//!
//! ## What the form asks, and where each answer comes from
//!
//! | source | lines |
//! |---|---|
//! | **the filer** ([`crate::tax::return_inputs::HsaInputs`]) | 1, 2, 4's gate, 10, 14b, 15, 17a/b's exception amount, Part III's gate |
//! | **the year's params** (§223(b)) | 3, 7 |
//! | **the documents** | 9 (Form W-2 box 12 code W), 14a (Form 1099-SA box 1) |
//! | **arithmetic the form states** | 5, 6, 8, 11, 12, 13, 14c, 16, 17b, 20, 21 |
//!
//! Nothing here is derived. Where the form says *"Subtract line 4 from line 3. If zero or less,
//! enter -0-"*, that is what the code does; where it says *"enter the smaller of"*, that is a `min`.
//!
//! ## ★★★ Where this build STOPS, and why each stop is the form's own sentence
//!
//! Four situations send the filer to paper btctax does not carry, and each REFUSES naming it rather
//! than printing a figure it cannot stand behind:
//!
//! | situation | the form's own words | refusal |
//! |---|---|---|
//! | not eligible every month with the same coverage, or enrolled in Medicare | line 3: *"All others, see the instructions for the amount to enter"* → the **Line 3 Limitation Chart and Worksheet** | [`RefuseReason::HsaLine3WorksheetRequired`] |
//! | both spouses have separate HSAs | *"Complete a separate Form 8889 for each spouse"* | [`RefuseReason::HsaSeparateForm8889Required`] |
//! | an Archer MSA or MA MSA | *"Before you begin: Complete Form 8853 … if required"* | [`RefuseReason::ArcherOrMaMsaNeedsForm8853`] |
//! | contributions over the limit | line 13: *"you may have to pay an additional tax on the excess contributions … See Form 5329"* | [`RefuseReason::HsaExcessContributionsNeedForm5329`] |
//! | a testing period failed | Part III line 18 needs the PRIOR year's Line 3 worksheet | [`RefuseReason::HsaTestingPeriodFailureNotComputed`] |
//!
//! ★ Every one of those is the UNDERSTATEMENT direction if guessed — a limit too high, a tax not
//! computed, income not included — which is why none of them is an advisory.
//!
//! [`RefuseReason::HsaLine3WorksheetRequired`]: crate::tax::return_refuse::RefuseReason::HsaLine3WorksheetRequired
//! [`RefuseReason::HsaSeparateForm8889Required`]: crate::tax::return_refuse::RefuseReason::HsaSeparateForm8889Required
//! [`RefuseReason::ArcherOrMaMsaNeedsForm8853`]: crate::tax::return_refuse::RefuseReason::ArcherOrMaMsaNeedsForm8853
//! [`RefuseReason::HsaExcessContributionsNeedForm5329`]: crate::tax::return_refuse::RefuseReason::HsaExcessContributionsNeedForm5329
//! [`RefuseReason::HsaTestingPeriodFailureNotComputed`]: crate::tax::return_refuse::RefuseReason::HsaTestingPeriodFailureNotComputed

use crate::conventions::Usd;
use crate::tax::return_inputs::{HdhpCoverage, ReturnInputs, SaAccountType};
use crate::tax::tables::HsaParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// **The Employer Contribution Worksheet** (`i8889--2024.txt:781-795`), transcribed.
///
/// ★★ It exists because a Form W-2 reports by CALENDAR year and Form 8889 line 9 wants the TAX
/// year's employer contributions. Its own heading: *"If either of the following apply, complete the
/// Employer Contribution Worksheet. • Employer contributions for 2023 are included on your 2024 Form
/// W-2, box 12, code W. • Employer contributions for 2024 are made in 2025."*
///
/// ★ Transcribed even when both adjustments are zero, because then line 5 = line 1 and the reader
/// can see WHY line 9 equals the W-2 total instead of having to trust that it does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmployerContributionWorksheet {
    /// **W1** — "Enter the employer contributions reported on your 2024 Form W-2, box 12, code W".
    pub line1_w2_box12_code_w: Usd,
    /// **W2** — "Enter employer contributions made in 2024 for tax year 2023".
    pub line2_made_this_year_for_prior_year: Usd,
    /// **W3** — "Subtract line 2 from line 1".
    pub line3: Usd,
    /// **W4** — "Enter employer contributions made in 2025 for tax year 2024".
    pub line4_made_next_year_for_this_year: Usd,
    /// **W5** — "Employer contributions for 2024. Add lines 3 and 4. Enter here and on your 2024
    /// Form 8889, line 9".
    pub line5: Usd,
}

/// **Form 8889 (2024 and 2025), transcribed** — Parts I, II and III, in the form's own numbering.
///
/// ★ `line17a` is a checkbox and `line1` a two-way one; every other field is money. The form prints
/// no line 14 (only 14a/14b/14c) and no line 17 (only 17a/17b), which is why those names are absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form8889 {
    // ── Part I — HSA Contributions and Deduction ─────────────────────────────────────────────────
    /// **L1** — "Check the box to indicate your coverage under a high-deductible health plan (HDHP)
    /// during 2024. See instructions" · `Self-only` / `Family`.
    pub line1_coverage: HdhpCoverage,
    /// **L2** — "HSA contributions you made for 2024 (or those made on your behalf), including those
    /// made by the unextended due date of your tax return that were for 2024. Do not include
    /// employer contributions, contributions through a cafeteria plan, or rollovers. See
    /// instructions".
    pub line2: Usd,
    /// **L3** — "If you were under age 55 at the end of 2024 and, on the first day of every month
    /// during 2024, you were, or were considered, an eligible individual with the same coverage,
    /// enter $4,150 ($8,300 for family coverage). All others, see the instructions for the amount to
    /// enter".
    pub line3: Usd,
    /// **L4** — "Enter the amount you and your employer contributed to your Archer MSAs for 2024
    /// from Form 8853, lines 1 and 2. If you or your spouse had family coverage under an HDHP at any
    /// time during 2024, also include any amount contributed to your spouse's Archer MSAs".
    pub line4: Usd,
    /// **L5** — "Subtract line 4 from line 3. If zero or less, enter -0-".
    pub line5: Usd,
    /// **L6** — "Enter the amount from line 5. But if you and your spouse each have separate HSAs
    /// and had family coverage under an HDHP at any time during 2024, see the instructions for the
    /// amount to enter".
    pub line6: Usd,
    /// **L7** — "If you were age 55 or older at the end of 2024, married, and you or your spouse had
    /// family coverage under an HDHP at any time during 2024, enter your additional contribution
    /// amount. See instructions".
    pub line7: Usd,
    /// **L8** — "Add lines 6 and 7".
    pub line8: Usd,
    /// **L9** — "Employer contributions made to your HSAs for 2024".
    pub line9: Usd,
    /// **L10** — "Qualified HSA funding distributions".
    pub line10: Usd,
    /// **L11** — "Add lines 9 and 10".
    pub line11: Usd,
    /// **L12** — "Subtract line 11 from line 8. If zero or less, enter -0-".
    pub line12: Usd,
    /// **L13** — "HSA deduction (see instructions)". · i8889: *"Generally, enter the smaller of line
    /// 2 or line 12 on line 13 and on Schedule 1 (Form 1040), Part II, line 13."*
    pub line13: Usd,
    // ── Part II — HSA Distributions ──────────────────────────────────────────────────────────────
    /// **L14a** — "Total distributions you received in 2024 from all HSAs (see instructions)".
    pub line14a: Usd,
    /// **L14b** — "Distributions included on line 14a that you rolled over to another HSA. Also
    /// include any excess contributions (and the earnings on those excess contributions) included on
    /// line 14a that were withdrawn by the due date of your return. See instructions".
    pub line14b: Usd,
    /// **L14c** — "Subtract line 14b from line 14a".
    pub line14c: Usd,
    /// **L15** — "Qualified medical expenses paid using HSA distributions (see instructions)".
    pub line15: Usd,
    /// **L16** — "Taxable HSA distributions. Subtract line 15 from line 14c. If zero or less, enter
    /// -0-. Also, include this amount in the total on Schedule 1 (Form 1040), Part I, line 8f".
    pub line16: Usd,
    /// **L17a** — "If any of the distributions included on line 16 meet any of the Exceptions to the
    /// Additional 20% Tax (see instructions), check here".
    pub line17a_exception_box: bool,
    /// **L17b** — "Additional 20% tax (see instructions). Enter 20% (0.20) of the distributions
    /// included on line 16 that are subject to the additional 20% tax. Also, include this amount in
    /// the total on Schedule 2 (Form 1040), Part II, line 17c".
    pub line17b: Usd,
    // ── Part III — Income and Additional Tax for Failure To Maintain HDHP Coverage ───────────────
    /// **L18** — "Last-month rule".
    pub line18: Usd,
    /// **L19** — "Qualified HSA funding distribution".
    pub line19: Usd,
    /// **L20** — "Total income. Add lines 18 and 19. Include this amount on Schedule 1 (Form 1040),
    /// Part I, line 8f".
    pub line20: Usd,
    /// **L21** — "Additional tax. Multiply line 20 by 10% (0.10). Include this amount in the total
    /// on Schedule 2 (Form 1040), Part II, line 17d".
    pub line21: Usd,
    /// The Employer Contribution Worksheet that produced [`Self::line9`]. Not a printed line of Form
    /// 8889 — the instructions' own *Keep for Your Records* worksheet, carried so line 9's
    /// provenance is visible rather than implied.
    pub employer_worksheet: EmployerContributionWorksheet,
}

impl Form8889 {
    /// **Schedule 1 line 8f** — *"Income from Form 8889"* (`f1040s1--2024.txt:30`).
    ///
    /// ★ TWO lines of Form 8889 reach it and the form says so twice: line 16 (*"Also, include this
    /// amount in the total on Schedule 1 (Form 1040), Part I, line 8f"*) and line 20 (*"Include this
    /// amount on Schedule 1 (Form 1040), Part I, line 8f"*). Reading only line 16 would drop the
    /// whole testing-period inclusion.
    #[must_use]
    pub fn schedule_1_line_8f(&self) -> Usd {
        self.line16 + self.line20
    }

    /// **Schedule 2 line 17c** — *"Additional tax on HSA distributions. Attach Form 8889"*
    /// (`f1040s2--2024.txt:84`). Form 8889 line 17b.
    #[must_use]
    pub fn schedule_2_line_17c(&self) -> Usd {
        self.line17b
    }

    /// **Schedule 2 line 17d** — *"Additional tax on an HSA because you didn't remain an eligible
    /// individual. Attach Form 8889"* (`f1040s2--2024.txt:87`). Form 8889 line 21.
    #[must_use]
    pub fn schedule_2_line_17d(&self) -> Usd {
        self.line21
    }

    /// **Schedule 1 line 13** — *"Health savings account deduction. Attach Form 8889"*
    /// (`f1040s1--2024.txt:62`). Form 8889 line 13.
    #[must_use]
    pub fn schedule_1_line_13(&self) -> Usd {
        self.line13
    }

    /// ★★★ **Must this Form 8889 be FILED?**
    ///
    /// It is not a threshold and not a heuristic: the form is required exactly when a §223 trigger
    /// fired, which is the question `hsa_activity` asks. Everything else — an all-zero Part II, a
    /// $0 deduction — is a legitimately blank part of a form that still files.
    ///
    /// ★ So this reads the DECLARATION, never the figures. A `must_file` computed from the numbers
    /// would drop the form for a filer whose contributions exactly equalled their employer's, and
    /// the IRS would receive a Schedule 1 line 13 with no attachment.
    #[must_use]
    pub fn must_file(ri: &ReturnInputs) -> bool {
        ri.sch1.hsa_activity == Some(true)
    }
}

/// ★ The **§223(b)(3)(B) additional contribution amount goes on line 3 or on line 7**, never both,
/// and the split is the instructions' own:
///
/// > *(6) If, at the end of 2024, you were age 55 or older and **unmarried or married with self-only
/// > HDHP coverage for the entire year**, you can increase the amount determined in (3) or (4) by
/// > $1,000 …*
/// > *Note. If you are **married and had family coverage at any time during the year**, the
/// > additional contribution amount is figured on line 7 and is not included on line 3.*
///
/// Returns `(added_to_line3, line7)`, of which at most one is non-zero.
fn additional_contribution_split(
    params: &HsaParams,
    age_55: bool,
    married: bool,
    coverage: HdhpCoverage,
) -> (Usd, Usd) {
    if !age_55 {
        return (Usd::ZERO, Usd::ZERO);
    }
    if married && coverage == HdhpCoverage::Family {
        // Line 7's own condition: age 55+, married, and family coverage at any time. Line 3's
        // "eligible every month with the same coverage" is what makes "at any time" cover the whole
        // year here, and the Additional Contribution Amount Worksheet's TIP then applies: "If items
        // (1) and (2) apply to all months during 2024, enter $1,000 on line 7."
        (Usd::ZERO, params.additional_contribution_55)
    } else {
        (params.additional_contribution_55, Usd::ZERO)
    }
}

/// ★★★ **The Form 1099-SA rows' box 1, summed** — Form 8889 line 14a's own source: *"Total
/// distributions you received in 2024 from all HSAs"*, and i8889 line 14a: *"These amounts should be
/// shown on Form 1099-SA, box 1."*
///
/// ★ Only rows whose box 5 says **HSA** are summed. An Archer MSA's or an MA MSA's distribution is
/// Form 8853's, and adding it here would put it on the wrong form — but the return refuses on such a
/// row before this is ever reached, so in practice this filter is the belt beside the braces.
#[must_use]
pub fn total_hsa_distributions(ri: &ReturnInputs) -> Usd {
    ri.sa_1099
        .iter()
        .filter(|r| r.box5_account_type == Some(SaAccountType::Hsa))
        .map(|r| r.box1_gross_distribution)
        .sum()
}

/// ★★★ **Form W-2 box 12 code W, summed** — Form 8889 line 9's own source: *"These contributions
/// should be shown on Form W-2, box 12, code W"* (`i8889--2024.txt:747-748`).
///
/// ★ **Read, never re-asked.** The figure is already on the return; a second question would invite
/// two answers to one question, and the one nobody typed would win silently.
#[must_use]
pub fn employer_contributions_from_w2s(ri: &ReturnInputs) -> Usd {
    ri.w2s
        .iter()
        .flat_map(|w| w.box12.iter())
        .filter(|e| e.code.eq_ignore_ascii_case("W"))
        .map(|e| e.amount)
        .sum()
}

/// Compute Form 8889 from the return's inputs, the documents it carries, and the year's §223(b)
/// figures.
///
/// **Every refusable situation has already been refused** by
/// [`crate::tax::return_refuse::screen_inputs`] before this runs, which is why this function has no
/// `Result`: it is the transcription, and the transcription of a return that cannot be filed is not
/// a thing anyone should see. The two answers it still has to READ — the coverage box and the
/// eligibility answer — are `Option<bool>` leaves whose `None` the same screen refuses, so the
/// `unwrap_or` fallbacks below are unreachable and are written in the SELF-ONLY / conservative
/// direction so that if they ever were reached they could not overstate a deduction.
#[must_use]
pub fn compute(ri: &ReturnInputs, params: &HsaParams) -> Form8889 {
    let h = &ri.hsa;
    let coverage = if h.family_coverage == Some(true) {
        HdhpCoverage::Family
    } else {
        HdhpCoverage::SelfOnly
    };
    let married = matches!(ri.filing_status, FilingStatus::Mfj | FilingStatus::Mfs);
    let base = match coverage {
        HdhpCoverage::SelfOnly => params.self_only_limit,
        HdhpCoverage::Family => params.family_limit,
    };
    let (line3_addition, line7) = additional_contribution_split(
        params,
        h.age_55_or_older_at_year_end == Some(true),
        married,
        coverage,
    );

    // ── Part I ───────────────────────────────────────────────────────────────────────────────────
    let line2 = h.line2_contributions_you_made;
    let line3 = base + line3_addition;
    // L4 — Archer MSA contributions come from Form 8853, which btctax does not build; the return
    // refuses when any exist, so the line is blank BY DECISION rather than by omission.
    let line4 = Usd::ZERO;
    let line5 = (line3 - line4).max(Usd::ZERO);
    // L6 — "Enter the amount from line 5." The allocation sentence that follows applies only when
    // both spouses have separate HSAs, which refuses.
    let line6 = line5;
    let line8 = line6 + line7;

    let employer_worksheet = {
        let w1 = employer_contributions_from_w2s(ri);
        let w2 = h.employer_contributions_prior_year;
        let w3 = w1 - w2;
        let w4 = h.employer_contributions_next_year;
        EmployerContributionWorksheet {
            line1_w2_box12_code_w: w1,
            line2_made_this_year_for_prior_year: w2,
            line3: w3,
            line4_made_next_year_for_this_year: w4,
            line5: w3 + w4,
        }
    };
    let line9 = employer_worksheet.line5;
    let line10 = h.line10_qualified_funding_distribution;
    let line11 = line9 + line10;
    let line12 = (line8 - line11).max(Usd::ZERO);
    // L13 — i8889: "Generally, enter the smaller of line 2 or line 12 on line 13". The "however"
    // that follows (line 2 above line 13 ⇒ excess contributions ⇒ Form 5329) is a REFUSAL, so by
    // the time this runs the smaller of the two is the whole rule.
    let line13 = line2.min(line12);

    // ── Part II ──────────────────────────────────────────────────────────────────────────────────
    let line14a = total_hsa_distributions(ri);
    let line14b = h.line14b_rollovers_and_withdrawn_excess;
    let line14c = line14a - line14b;
    let line15 = h.line15_qualified_medical_expenses;
    let line16 = (line14c - line15).max(Usd::ZERO);
    // L17a — the form's own sentence: "If any of the distributions included on line 16 meet any of
    // the Exceptions to the Additional 20% Tax … check here."
    let excepted = h.line16_amount_meeting_an_exception.min(line16);
    let line17a_exception_box = excepted > Usd::ZERO;
    // L17b — "Enter 20% (0.20) of the distributions included on line 16 that are subject to the
    // additional 20% tax", i.e. of line 16 LESS the part that meets an exception.
    let line17b = dec!(0.20) * (line16 - excepted);

    // ── Part III ─────────────────────────────────────────────────────────────────────────────────
    // Both lines require the PRIOR year's Line 3 Limitation Chart and Worksheet, which btctax
    // carries for no year, so `testing_period_failure == Some(true)` REFUSES upstream. Reaching here
    // means the filer answered NO, and the correct Part III is empty.
    let line18 = Usd::ZERO;
    let line19 = Usd::ZERO;
    let line20 = line18 + line19;
    let line21 = dec!(0.10) * line20;

    Form8889 {
        line1_coverage: coverage,
        line2,
        line3,
        line4,
        line5,
        line6,
        line7,
        line8,
        line9,
        line10,
        line11,
        line12,
        line13,
        line14a,
        line14b,
        line14c,
        line15,
        line16,
        line17a_exception_box,
        line17b,
        line18,
        line19,
        line20,
        line21,
        employer_worksheet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::document_census::DocumentRow;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::return_inputs::{Box12Entry, Form1099Sa, Form5498Sa, Owner, W2};
    use crate::tax::return_refuse::{screen_inputs, RefuseReason};
    use crate::tax::testonly::{answer_all_live_declarations, ty2024_params, ty2024_table};

    use rust_decimal_macros::dec;

    /// A single filer with one W-2 whose employer put $1,000 in box 12 code W, an HSA trigger
    /// affirmed, and every Form 8889 question answered the way an ordinary self-only holder answers
    /// it. Every kill below starts here and changes ONE thing.
    fn hsa_household() -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            tax_year: 2024,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                employer: "SCHOOL DISTRICT".into(),
                box1_wages: dec!(60000),
                box2_fed_withheld: dec!(6000),
                box3_ss_wages: dec!(60000),
                box4_ss_withheld: dec!(3720),
                box5_medicare_wages: dec!(60000),
                box6_medicare_withheld: dec!(870),
                box12: vec![Box12Entry {
                    code: "W".into(),
                    amount: dec!(1000),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        ri.header.taxpayer.first_name = "Pat".into();
        ri.header.taxpayer.last_name = "Roe".into();
        ri.header.taxpayer.ssn = "222334444".into();
        ri.sch1.hsa_activity = Some(true);
        ri.hsa.family_coverage = Some(false);
        ri.hsa.eligible_every_month_same_coverage = Some(true);
        ri.hsa.age_55_or_older_at_year_end = Some(false);
        ri.hsa.enrolled_in_medicare_any_month = Some(false);
        ri.hsa.both_spouses_have_hsas = Some(false);
        ri.hsa.archer_msa_activity = Some(false);
        ri.hsa.testing_period_failure = Some(false);
        ri.hsa.line2_contributions_you_made = dec!(2000);
        answer_all_live_declarations(&mut ri);
        // `answer_all_live_declarations` answers every census row at its neutral (`false`); the W-2
        // row must say what the return actually holds.
        ri.documents.set(DocumentRow::W2, Some(true));
        ri
    }

    fn refuse(ri: &ReturnInputs) -> Option<RefuseReason> {
        screen_inputs(ri, &ty2024_table(), &ty2024_params()).map(|r| r.reason)
    }

    fn form(ri: &ReturnInputs) -> Form8889 {
        compute(ri, &ty2024_params().hsa)
    }

    /// ★★★ **THE CONTRIBUTION LIMIT COMES FROM THE YEAR'S PARAMS, AND A CONTRIBUTION OVER IT
    ///     REFUSES NAMING FORM 5329** — the brief's params kill.
    ///
    /// Line 3 is the TY2024 §223(b)(2)(A) self-only limitation, $4,150 (Rev. Proc. 2023-23
    /// §2.01(1)), which is the figure Form 8889 (2024) line 3 prints. At the limit the return files;
    /// one dollar over it, line 2 exceeds line 13 and the instructions' own sentence takes over:
    /// *"you or someone on your behalf (or your employer) contributed more to your HSA than is
    /// allowable … See Form 5329 … to figure the additional tax."*
    ///
    /// ★ The kill CALLS the instrument: it reads the LIMIT off `ty2024_params().hsa`, so a params
    /// edit that moved the figure would move this assertion with it — and it plants the defect
    /// (a contribution over the limit) rather than asserting a constant.
    #[test]
    fn a_contribution_over_the_years_limit_refuses_naming_form_5329() {
        let p = ty2024_params();
        let limit = p.hsa.self_only_limit;
        assert_eq!(
            limit,
            dec!(4150),
            "TY2024 §223(b)(2)(A), Rev. Proc. 2023-23"
        );

        // Employer contributions eat into the same limit, so the filer's own room is limit − $1,000.
        let room = limit - dec!(1000);
        let mut at = hsa_household();
        at.hsa.line2_contributions_you_made = room;
        assert_eq!(refuse(&at), None, "a contribution AT the limit files");
        let f = form(&at);
        assert_eq!(f.line3, limit, "line 3 is the year's own §223(b) figure");
        assert_eq!(f.line13, room, "the whole contribution is deductible");

        // One dollar over, and the form's own sentence takes over.
        let mut over = hsa_household();
        over.hsa.line2_contributions_you_made = room + dec!(1);
        assert_eq!(
            refuse(&over),
            Some(RefuseReason::HsaExcessContributionsNeedForm5329),
            "a contribution over the limit must refuse naming Form 5329"
        );
        let f = form(&over);
        assert!(
            f.line2 > f.line13,
            "the EXCESS is visible on the form's own lines: line 2 {} vs line 13 {}",
            f.line2,
            f.line13
        );
    }

    /// ★★★ **A DISTRIBUTION NOT USED FOR QUALIFIED MEDICAL EXPENSES REACHES BOTH SCHEDULE 1 LINE 8f
    ///     AND SCHEDULE 2 LINE 17c** — the brief's reach kill, on one fixture.
    ///
    /// $3,000 out of the account, $1,200 of it spent on qualified medical expenses: line 16 is the
    /// $1,800 remainder (income), and line 17b is 20% of it ($360, the §223(f)(4) additional tax).
    /// Both figures then have to arrive on the printed return — a form that computed them and a
    /// Schedule 1 / Schedule 2 that did not read them is the "figure with no reader" defect.
    #[test]
    fn an_unqualified_distribution_reaches_schedule_1_line_8f_and_schedule_2_line_17c() {
        let mut ri = hsa_household();
        ri.documents.set(DocumentRow::Sa1099, Some(true));
        ri.sa_1099.push(Form1099Sa {
            payer: "HSA TRUSTEE".into(),
            box1_gross_distribution: dec!(3000),
            box3_distribution_code: "1".into(),
            box5_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        });
        ri.hsa.line15_qualified_medical_expenses = dec!(1200);
        assert_eq!(refuse(&ri), None, "this return files");

        let f = form(&ri);
        assert_eq!(f.line14a, dec!(3000), "line 14a sums Form 1099-SA box 1");
        assert_eq!(f.line16, dec!(1800), "14c − 15, the taxable remainder");
        assert_eq!(f.line17b, dec!(360), "20% of line 16 (§223(f)(4))");
        assert_eq!(f.schedule_1_line_8f(), dec!(1800));
        assert_eq!(f.schedule_2_line_17c(), dec!(360));

        // …and they REACH the printed return, which is the half a form-only assertion misses.
        let ar = assemble_absolute(
            &ri,
            &Default::default(),
            &ty2024_params(),
            &ty2024_table(),
            2024,
        );
        let s1 = crate::tax::printed::schedule_1_lines(&ar).expect("Schedule 1 files");
        assert_eq!(s1.line8f, dec!(1800), "Schedule 1 line 8f carries line 16");
        let f8959 = crate::tax::other_taxes::form_8959_lines(
            ri.filing_status,
            dec!(60000),
            dec!(870),
            None,
        );
        let s2 = crate::tax::printed::schedule_2_lines(None, &f8959, None, None, Some(&f))
            .expect("Schedule 2 files on an HSA additional tax alone");
        assert_eq!(
            s2.line17c,
            Some(dec!(360)),
            "Schedule 2 line 17c carries line 17b"
        );
        assert_eq!(s2.line18, dec!(360), "line 18 adds 17a through 17z");
        assert_eq!(
            s2.line21,
            dec!(360),
            "line 21 sums line 18 into total other taxes → 1040 line 23"
        );
    }

    /// ★★★ **THE EXCEPTIONS CHECKBOX IS THE FORM'S OWN SENTENCE, AND IT SPLITS LINE 17b.**
    ///
    /// *"Enter on line 17b only 20% (0.20) of any amount included on line 16 that does not meet any
    /// of the exceptions."* With $800 of the $1,800 meeting one (the beneficiary turned 65 mid-year),
    /// the box is checked and the tax is 20% of the remaining $1,000.
    #[test]
    fn the_line_17a_box_is_checked_by_the_excepted_amount_and_17b_taxes_the_rest() {
        let mut ri = hsa_household();
        ri.documents.set(DocumentRow::Sa1099, Some(true));
        ri.sa_1099.push(Form1099Sa {
            payer: "HSA TRUSTEE".into(),
            box1_gross_distribution: dec!(3000),
            box5_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        });
        ri.hsa.line15_qualified_medical_expenses = dec!(1200);
        assert!(!form(&ri).line17a_exception_box, "no exception, no box");

        ri.hsa.line16_amount_meeting_an_exception = dec!(800);
        let f = form(&ri);
        assert!(f.line17a_exception_box, "an excepted amount checks the box");
        assert_eq!(f.line17b, dec!(200), "20% of 1,800 − 800");
    }

    /// ★★★ **A DECLARED FORM 1099-SA WITH NO ROW REFUSES** — the answered-ness invariant at the
    ///     document level, and the brief's `DocumentDeclaredNotTranscribed` kill.
    ///
    /// A distribution is gross income plus a 20% additional tax under §223(f); a census row saying
    /// *"I received one"* beside zero transcribed rows is exactly *"nothing ever populated it"*.
    /// One row, and the return files with Part II filled.
    #[test]
    fn a_declared_1099_sa_with_no_row_refuses_and_one_row_files() {
        let mut ri = hsa_household();
        ri.documents.set(DocumentRow::Sa1099, Some(true));
        assert_eq!(
            refuse(&ri),
            Some(RefuseReason::DocumentDeclaredNotTranscribed {
                kind: DocumentRow::Sa1099
            }),
            "a declared 1099-SA with no row must refuse"
        );

        ri.sa_1099.push(Form1099Sa {
            payer: "HSA TRUSTEE".into(),
            box1_gross_distribution: dec!(500),
            box5_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        });
        assert_eq!(refuse(&ri), None, "one transcribed row files");
        assert_eq!(form(&ri).line14a, dec!(500), "and Part II is FILLED");
    }

    /// ★★★ **THE CONTRADICTION IN BOTH DIRECTIONS.** A census `No` beside a transcribed row is a
    ///     document the filer says they do not hold, and it refuses on both HSA rows.
    #[test]
    fn a_no_beside_a_transcribed_hsa_row_refuses_the_contradiction() {
        let mut ri = hsa_household();
        ri.documents.set(DocumentRow::Sa1099, Some(false));
        ri.sa_1099.push(Form1099Sa {
            payer: "HSA TRUSTEE".into(),
            box1_gross_distribution: dec!(500),
            box5_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        });
        assert_eq!(
            refuse(&ri),
            Some(RefuseReason::DocumentCensusContradicted {
                kind: DocumentRow::Sa1099
            })
        );

        let mut ri = hsa_household();
        ri.documents.set(DocumentRow::Sa5498, Some(false));
        ri.sa_5498.push(Form5498Sa {
            trustee: "HSA TRUSTEE".into(),
            box2_total_contributions: dec!(2000),
            box6_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        });
        assert_eq!(
            refuse(&ri),
            Some(RefuseReason::DocumentCensusContradicted {
                kind: DocumentRow::Sa5498
            })
        );
    }

    /// ★★★ **AN UNTRANSCRIBED BOX 5 REFUSES RATHER THAN DEFAULTING TO "HSA"**, and an Archer MSA
    ///     refuses naming Form 8853 — the two halves of the checkbox rule.
    #[test]
    fn the_account_type_checkbox_refuses_unanswered_and_refuses_an_archer_msa() {
        let mut blank = hsa_household();
        blank.documents.set(DocumentRow::Sa1099, Some(true));
        blank.sa_1099.push(Form1099Sa {
            payer: "HSA TRUSTEE".into(),
            box1_gross_distribution: dec!(500),
            ..Default::default() // box 5 NOT transcribed
        });
        assert!(
            matches!(
                refuse(&blank),
                Some(RefuseReason::SaAccountTypeNotTranscribed(_))
            ),
            "an untranscribed box 5 must refuse, never default to HSA: {:?}",
            refuse(&blank)
        );

        let mut archer = hsa_household();
        archer.documents.set(DocumentRow::Sa1099, Some(true));
        archer.sa_1099.push(Form1099Sa {
            payer: "MSA TRUSTEE".into(),
            box1_gross_distribution: dec!(500),
            box5_account_type: Some(SaAccountType::ArcherMsa),
            ..Default::default()
        });
        let r = refuse(&archer);
        assert!(
            matches!(r, Some(RefuseReason::ArcherOrMaMsaNeedsForm8853(_))),
            "an Archer MSA must refuse naming Form 8853: {r:?}"
        );
    }

    /// ★★★ **THE EMPLOYER'S CONTRIBUTION IS READ FROM THE W-2, NEVER RE-ASKED**, and the Employer
    ///     Contribution Worksheet is what reconciles the CALENDAR year to the TAX year.
    #[test]
    fn line_9_reads_w2_box_12_code_w_through_the_worksheet() {
        let ri = hsa_household();
        let f = form(&ri);
        assert_eq!(
            f.employer_worksheet.line1_w2_box12_code_w,
            dec!(1000),
            "worksheet line 1 is the W-2's box 12 code W"
        );
        assert_eq!(f.line9, dec!(1000), "and line 9 is the worksheet's line 5");
        assert_eq!(f.line12, f.line8 - f.line9, "line 12 subtracts line 11");

        // The two calendar-year adjustments move line 9, and nothing else does.
        let mut adj = hsa_household();
        adj.hsa.employer_contributions_prior_year = dec!(300);
        adj.hsa.employer_contributions_next_year = dec!(50);
        let f = form(&adj);
        assert_eq!(f.employer_worksheet.line3, dec!(700), "1,000 − 300");
        assert_eq!(f.employer_worksheet.line5, dec!(750), "700 + 50");
        assert_eq!(f.line9, dec!(750));
    }

    /// ★★★ **THE §223(b)(3)(B) $1,000 GOES ON LINE 3 OR LINE 7, NEVER BOTH** — the instructions'
    ///     own split, which a single "add $1,000" would collapse.
    #[test]
    fn the_age_55_amount_lands_on_line_3_or_line_7_by_marital_status_and_coverage() {
        let p = ty2024_params();
        // Unmarried, 55+ ⇒ line 3 carries it, line 7 is blank.
        let mut single = hsa_household();
        single.hsa.age_55_or_older_at_year_end = Some(true);
        let f = form(&single);
        assert_eq!(f.line3, p.hsa.self_only_limit + dec!(1000));
        assert_eq!(f.line7, Usd::ZERO, "line 7 is for the MARRIED family case");

        // Married with FAMILY coverage, 55+ ⇒ line 7 carries it and line 3 does not.
        let mut married = hsa_household();
        married.filing_status = FilingStatus::Mfj;
        married.hsa.age_55_or_older_at_year_end = Some(true);
        married.hsa.family_coverage = Some(true);
        answer_all_live_declarations(&mut married);
        married.documents.set(DocumentRow::W2, Some(true));
        let f = form(&married);
        assert_eq!(
            f.line3, p.hsa.family_limit,
            "line 3 is the bare family limit"
        );
        assert_eq!(f.line7, dec!(1000), "the additional amount is on line 7");
        assert_eq!(f.line8, f.line6 + f.line7, "and line 8 adds them");
    }

    /// ★★★ **PART III REFUSES RATHER THAN PRINTING ZEROS**, because line 18 needs a PRIOR year's
    ///     Line 3 Limitation Chart and Worksheet, which btctax carries for no year at all.
    #[test]
    fn a_testing_period_failure_refuses_naming_the_prior_years_worksheet() {
        let mut ri = hsa_household();
        ri.hsa.testing_period_failure = Some(true);
        assert_eq!(
            refuse(&ri),
            Some(RefuseReason::HsaTestingPeriodFailureNotComputed)
        );
        // Answered NO, Part III is empty and that is the CORRECT Part III.
        let f = form(&hsa_household());
        assert_eq!((f.line18, f.line19, f.line20, f.line21), Default::default());
    }

    /// ★★★ **PART III'S TWO REACHES ARE PINNED ON A CONSTRUCTED FORM, and that is deliberate.**
    ///
    /// ★★ **The gap this closes was found by planting it.** `compute` can never produce a non-zero
    /// Part III — a testing-period failure REFUSES — so deleting `+ self.line20` from
    /// [`Form8889::schedule_1_line_8f`] left all eleven other kills green. A term that no test can
    /// distinguish from its absence is a guarantee that does not exist, and Part III is exactly
    /// where a future build that DOES carry the prior year's Line 3 worksheet will start.
    ///
    /// So this test builds the struct directly, which is the only way to reach the branch today. It
    /// asserts what the FORM says on its own two lines: *"Total income. Add lines 18 and 19. Include
    /// this amount on Schedule 1 (Form 1040), Part I, line 8f"* and *"Additional tax. Multiply line
    /// 20 by 10% (0.10). Include this amount in the total on Schedule 2 (Form 1040), Part II, line
    /// 17d"* — so line 8f is the SUM of two lines, not one.
    #[test]
    fn part_iii_reaches_schedule_1_line_8f_and_schedule_2_line_17d() {
        let f = Form8889 {
            line1_coverage: HdhpCoverage::SelfOnly,
            line16: dec!(1800),
            line17b: dec!(360),
            line18: dec!(2500),
            line19: dec!(1000),
            line20: dec!(3500),
            line21: dec!(350),
            ..zero_form()
        };
        assert_eq!(
            f.schedule_1_line_8f(),
            dec!(5300),
            "line 8f is line 16 PLUS line 20 — the form routes both there, and reading only one \
             drops the whole testing-period inclusion from income"
        );
        assert_eq!(f.schedule_2_line_17c(), dec!(360), "17c is line 17b");
        assert_eq!(f.schedule_2_line_17d(), dec!(350), "17d is line 21");
    }

    /// An all-zero Form 8889 to build variants from — the struct has no `Default` on purpose (line 1
    /// is a coverage box, and there is no default coverage).
    fn zero_form() -> Form8889 {
        Form8889 {
            line1_coverage: HdhpCoverage::SelfOnly,
            line2: Usd::ZERO,
            line3: Usd::ZERO,
            line4: Usd::ZERO,
            line5: Usd::ZERO,
            line6: Usd::ZERO,
            line7: Usd::ZERO,
            line8: Usd::ZERO,
            line9: Usd::ZERO,
            line10: Usd::ZERO,
            line11: Usd::ZERO,
            line12: Usd::ZERO,
            line13: Usd::ZERO,
            line14a: Usd::ZERO,
            line14b: Usd::ZERO,
            line14c: Usd::ZERO,
            line15: Usd::ZERO,
            line16: Usd::ZERO,
            line17a_exception_box: false,
            line17b: Usd::ZERO,
            line18: Usd::ZERO,
            line19: Usd::ZERO,
            line20: Usd::ZERO,
            line21: Usd::ZERO,
            employer_worksheet: EmployerContributionWorksheet::default(),
        }
    }

    /// ★★★ **THE LINE 3 WORKSHEET AND THE SEPARATE-FORM RULE**, the two remaining stops.
    #[test]
    fn part_year_eligibility_medicare_and_two_spouses_each_refuse_naming_where_to_go() {
        for (label, mutate) in [
            (
                "part-year eligibility",
                (|r: &mut ReturnInputs| r.hsa.eligible_every_month_same_coverage = Some(false))
                    as fn(&mut ReturnInputs),
            ),
            ("Medicare", |r: &mut ReturnInputs| {
                r.hsa.enrolled_in_medicare_any_month = Some(true)
            }),
        ] {
            let mut ri = hsa_household();
            mutate(&mut ri);
            assert_eq!(
                refuse(&ri),
                Some(RefuseReason::HsaLine3WorksheetRequired),
                "{label} must send the filer to the Line 3 Limitation Chart and Worksheet"
            );
        }
        let mut both = hsa_household();
        both.hsa.both_spouses_have_hsas = Some(true);
        assert_eq!(
            refuse(&both),
            Some(RefuseReason::HsaSeparateForm8889Required)
        );
    }

    /// ★★★ **`must_file` READS THE DECLARATION, NOT THE FIGURES** — the defect a threshold would
    ///     introduce, planted.
    ///
    /// A filer whose employer contributed exactly the whole limit has a $0 line 13, a $0 line 16 and
    /// a $0 line 17b: every figure on the form is zero, and the form is still required.
    #[test]
    fn an_all_zero_form_8889_still_files_because_the_declaration_says_so() {
        let mut ri = hsa_household();
        ri.hsa.line2_contributions_you_made = Usd::ZERO;
        assert_eq!(refuse(&ri), None);
        let f = form(&ri);
        assert_eq!(f.line13, Usd::ZERO, "no deduction");
        assert_eq!(f.line16, Usd::ZERO, "no taxable distribution");
        assert!(
            Form8889::must_file(&ri),
            "an all-zero Form 8889 is still REQUIRED — the trigger fired"
        );

        // …and the same figures with the declaration answered NO file no form at all.
        let mut no = ri.clone();
        no.sch1.hsa_activity = Some(false);
        assert!(!Form8889::must_file(&no));
    }

    /// ★★★ **AN EMPLOYER CONTRIBUTION WITH NO HSA ACTIVITY IS A CONTRADICTION.** The W-2 says money
    ///     went into an HSA; the declaration says nothing happened. Before T16, code W refused as an
    ///     unsupported box-12 code — the correct answer while no form read it, and the wrong one now.
    #[test]
    fn a_w2_code_w_beside_a_no_hsa_declaration_refuses_the_contradiction() {
        let mut ri = hsa_household();
        ri.sch1.hsa_activity = Some(false);
        assert_eq!(
            refuse(&ri),
            Some(RefuseReason::HsaEmployerContributionWithoutActivity),
            "the W-2 and the declaration cannot both be true"
        );
    }
}
