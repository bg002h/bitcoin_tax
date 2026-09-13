//! **The §111(a) STATE AND LOCAL INCOME TAX REFUND WORKSHEET — Schedule 1, Line 1**, transcribed.
//!
//! ★★★ **WHY THIS EXISTS (FR-196).** An itemizer in a state that taxes income cannot file at all:
//! `RefuseReason::StateAndLocalRefundWorksheetNotComputed` refuses the whole return at `income
//! import`, and it is reached by *"the definition of a second year of itemizing in a state with
//! income tax."* The refusal is honest — §111(a)'s tax-benefit rule makes some or all of the refund
//! income on Schedule 1 line 1, the worksheet decides how much, and btctax modelled no part of it —
//! but it is a wall, not an answer. This module is the answer.
//!
//! **Transcribed, not derived** (`CLAUDE.md`): one field per numbered line, named for the line,
//! carrying the official instruction text verbatim as its doc comment, in the form's own numbering.
//! No closed form. The two STOPs and the MFS skip are modelled as SKIPS — an `Option<Usd>` that is
//! `None` because the form sent the filer past the line, which is not the same fact as `-0-`.
//!
//! **THE AUTHORITY IS `i1040gi` FOR THE FILING YEAR, AND THE PRIOR YEAR IS BAKED INTO ITS TEXT.**
//! The worksheet printed in the instructions for year *Y* figures how much of a refund received in
//! *Y* — of tax paid in *Y−1* — is income in *Y*. Every one of its lines reads *"your **Y−1**
//! Schedule A"*, and its lines 5 and 6 print **Y−1's** standard-deduction figures. So the worksheet
//! is per REVISION, and [`REVISIONS`] carries one entry per archived `i1040gi` revision rather than
//! one hardcoded year — the T8 defect (*"a fixture literal hardcoded to `year: 2024`"*) in the shape
//! it would take here. [`tests::every_archived_revision_is_transcribed`] derives the expected
//! revision set from the extract DIRECTORY, so archiving `i1040gi--2026.txt` reds this module until
//! it is transcribed.
//!
//! ★ **A year with no archived revision REFUSES** ([`NotUsable::NoArchivedRevision`]). It does not
//! fall back to the nearest revision: the constants on lines 5 and 6 are *that year's*, and using a
//! neighbour's would be a silently wrong figure in whichever direction the indexing happened to run.
//!
//! ★★ **Two transcription facts worth recording, because both are the Form 6251 line-33 class** —
//! a cross-reference read off a rendered page instead of the text layer:
//!
//! 1. Line 1's cap is *"the amount of your state and local income taxes shown on your Y−1 Schedule
//!    A, **line 5d**"* — and 5d is the TOTAL of 5a+5b+5c, not the income-tax component alone. The
//!    text says 5d; this module says 5d. (It is also the conservative direction: a larger cap can
//!    only make line 1, and hence the taxable part, larger.)
//! 2. Exception 7 cites *"your Y−1 Form 1040 or 1040-SR, **line 18**"* in the 2025 revision and
//!    *"**line 16**"* in the 2024 one — the same sentence, a different line, because the 1040
//!    renumbered underneath it. A single shared quote would have been wrong for one of the two.
//!
//! ★★★ **THE WORKSHEET IS NOT ALWAYS USABLE, AND THAT IS ITS OWN FIRST INSTRUCTION.** *"Before you
//! begin: Be sure you have read the Exception in the instructions for this line to see if you can
//! use this worksheet instead of Pub. 525…"* — and the Exception lists **nine** conditions under any
//! of which the filer must use *Itemized Deduction Recoveries* in Pub. 525 instead. btctax models no
//! part of Pub. 525, so an affirmed exception is a REFUSAL ([`NotUsable::Pub525Exception`]), not a
//! worksheet run. [`Pub525Exception::ALL`] and the `_`-free match in
//! [`StateLocalRefundFacts::exception_that_applies`] are what keep a tenth condition from being
//! silently dropped, and [`tests::the_exception_numbers_are_read_off_the_form`] reads the set off
//! the extract.
//!
//! **The TIP is an exit, not commentary.** *"None of your refund is taxable if, in the year you paid
//! the tax, you either (a) didn't itemize deductions, or (b) elected to deduct state and local
//! general sales taxes instead of state and local income taxes."* Limb (a) is the question btctax
//! already asks (`ReturnInputs::itemized_prior_year`); limb (b) **was never asked**, and a filer who
//! elected sales taxes owes nothing on the refund. Both are [`TipLimb`], and both produce a Schedule
//! 1 line 1 that is **blank by decision**.

use crate::conventions::Usd;
use crate::tax::types::FilingStatus;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The revision table — one entry per archived `i1040gi` revision.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The `design/forms/extract/` stem every revision is read from, relative to the repo root.
///
/// ★ The quotes travel as DATA rather than as an `include_str!` for `capital_loss_carryover`'s
/// reason: `design/forms/extract/` is outside this published crate, and a reach out of the crate
/// root ships a tarball that builds in the workspace and is broken for everyone else, **with exit
/// 0**. The checking lives in this module's `#[cfg(test)]` block, which may read the repo tree
/// (`return_refuse.rs`'s `tests::repo_root` is the precedent) and is not compiled into the tarball.
pub const EXTRACT_STEM: &str = "design/forms/extract/i1040gi--";

/// One line-5 bullet: the filing statuses it names, the amount it prints, and the bullet verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line5Bullet {
    /// The statuses this bullet covers. Every [`FilingStatus`] is covered by exactly one bullet,
    /// asserted by [`tests::every_filing_status_has_exactly_one_line5_bullet`] — the form's
    /// *"Married filing jointly or qualifying surviving spouse"* is what puts `Qss` with `Mfj`.
    pub statuses: &'static [FilingStatus],
    /// The whole-dollar amount the bullet prints. Cross-checked against [`Self::printed`] by
    /// [`tests::every_printed_amount_agrees_with_the_figure_beside_it`], so the pair cannot drift.
    pub amount: u32,
    /// The bullet as the form prints it, verbatim (bullet glyph and em dash included).
    pub printed: &'static str,
}

/// One archived revision of the worksheet: the year whose `i1040gi` prints it, and every quote and
/// constant that revision carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision {
    /// The tax year these instructions are FOR — the year the refund was RECEIVED, and the year
    /// whose Schedule 1 line 1 this worksheet fills.
    pub year: i32,
    /// *"Before you begin"* — the worksheet's own first instruction.
    pub before_you_begin: &'static str,
    /// The TIP above the Exception, which is the worksheet's other exit.
    pub tip: &'static str,
    /// The Exception's own header sentence.
    pub exception_header: &'static str,
    /// The nine Exception conditions, `(number, text)`, in the form's order.
    pub exceptions: &'static [(u8, &'static str)],
    /// The nine numbered worksheet lines, `(number, text)`.
    pub lines: &'static [(u8, &'static str)],
    /// Line 2's *"No."* branch — the one that SKIPS line 2 and line 3's comparison.
    pub line2_no: &'static str,
    /// Line 2's *"Yes."* branch.
    pub line2_yes: &'static str,
    /// Line 3's *"Yes."* branch.
    pub line3_yes: &'static str,
    /// The STOP text both line 3 and line 8 reach — *"None of your refund is taxable."*
    pub stop: &'static str,
    /// The Note that skips lines 5 through 7 for a prior-year MFS filer whose spouse itemized.
    pub mfs_note: &'static str,
    /// Line 5's three bullets.
    pub line5_bullets: &'static [Line5Bullet],
    /// Line 6's four checkbox labels, in the form's order: taxpayer aged, taxpayer blind, spouse
    /// aged, spouse blind. ★ The aged labels carry a DATE that moves every revision.
    pub line6_boxes: &'static [&'static str],
    /// Line 6's *"No boxes checked. Enter -0-."*
    pub line6_no_boxes: &'static str,
    /// Line 6's multiply instruction, verbatim — the source of both per-box amounts.
    pub line6_multiply: &'static str,
    /// The per-box amount for a prior-year MFJ / MFS / QSS filer (§63(f)'s married figure).
    pub line6_per_box_married: u32,
    /// The per-box amount *"if your Y−1 filing status was single or head of household"*.
    pub line6_per_box_unmarried: u32,
    /// Line 6's footnote — the three conditions a prior-year MFS filer must meet to check the
    /// SPOUSE boxes at all.
    pub line6_mfs_footnote: &'static str,
    /// Line 8's *"Yes."* branch.
    pub line8_yes: &'static str,
}

impl Revision {
    /// The year the tax was PAID — the year every *"your Y−1 Schedule A"* reference names.
    ///
    /// ★ Derived, never a second field: a revision whose two years disagreed would be a silent
    /// contradiction, and [`Pub525Exception::E1RefundForAnotherYear`] is the condition that makes
    /// the relationship load-bearing (*"You received a refund in Y that is for a tax year other
    /// than Y−1"*).
    #[must_use]
    pub const fn prior_year(&self) -> i32 {
        self.year - 1
    }

    /// The extract this revision's every quote is checked against.
    #[must_use]
    pub fn source_extract(&self) -> String {
        format!("{EXTRACT_STEM}{}.txt", self.year)
    }

    /// Line 5 — the prior-year basic standard deduction the form PRINTS for a filing status.
    ///
    /// `None` is unreachable given [`tests::every_filing_status_has_exactly_one_line5_bullet`], and
    /// it is an `Option` rather than a panic because this crate's compute core does not panic.
    #[must_use]
    pub fn line5_amount(&self, prior_status: FilingStatus) -> Option<Usd> {
        self.line5_bullets
            .iter()
            .find(|b| b.statuses.contains(&prior_status))
            .map(|b| Usd::from(b.amount))
    }

    /// Line 6 — the per-box amount, *"($1,950 if your Y−1 filing status was single or head of
    /// household)"*.
    #[must_use]
    pub fn line6_per_box(&self, prior_status: FilingStatus) -> Usd {
        match prior_status {
            FilingStatus::Single | FilingStatus::HoH => Usd::from(self.line6_per_box_unmarried),
            // ★ `Qss` takes the MARRIED figure: the parenthetical names single and head of
            //   household ONLY, and §63(f)(1) keys the larger amount to being unmarried and not a
            //   surviving spouse. An `_` here would have hidden that decision.
            FilingStatus::Mfj | FilingStatus::Mfs | FilingStatus::Qss => {
                Usd::from(self.line6_per_box_married)
            }
        }
    }

    /// This revision's text for one Exception condition.
    #[must_use]
    pub fn exception_text(&self, which: Pub525Exception) -> Option<&'static str> {
        self.exceptions
            .iter()
            .find(|(n, _)| *n == which.number())
            .map(|(_, t)| *t)
    }
}

/// The archived revisions, oldest first.
///
/// ★★ **Derived-set discipline**: [`tests::every_archived_revision_is_transcribed`] enumerates the
/// `i1040gi--*.txt` extracts that actually contain the worksheet and requires a `Revision` for each.
/// A new archive reds this module rather than silently widening the set of years that refuse.
pub const REVISIONS: &[Revision] = &[REVISION_2024, REVISION_2025];

/// The revision that figures a refund received in `tax_year`, or `None` when none is archived.
#[must_use]
pub fn revision_for(tax_year: i32) -> Option<&'static Revision> {
    REVISIONS.iter().find(|r| r.year == tax_year)
}

/// **TY2024** — `design/forms/extract/i1040gi--2024.txt`, worksheet at 41290-41386, line-1
/// instructions at 41101-41171. Figures a refund received in 2024 of tax paid in 2023.
pub const REVISION_2024: Revision = Revision {
    year: 2024,
    before_you_begin: "Be sure you have read the Exception in the instructions for this line to see \
                       if you can use this worksheet instead of Pub. 525 to figure if any of your \
                       refund is taxable.",
    tip: "if, in the year you paid the tax, you either (a) didn't itemize deductions, or (b) \
          elected to deduct state and local general sales taxes instead of state and local income \
          taxes.",
    exception_header: "Exception. See Itemized Deduction Recoveries in Pub. 525 instead of using \
                       the State and Local Income Tax Refund Worksheet in these instructions if \
                       any of the following applies.",
    exceptions: &[
        (
            1,
            "You received a refund in 2024 that is for a tax year other than 2023.",
        ),
        (
            2,
            "You received a refund other than an income tax refund, such as a general sales tax or \
             real property tax refund, in 2024 of an amount deducted or credit claimed in an \
             earlier year.",
        ),
        (
            3,
            "You had taxable income on your 2023 Form 1040 or 1040-SR, line 15, but no tax on your \
             Form 1040 or 1040-SR, line 16, because of the 0% tax rate on net capital gain and \
             qualified dividends in certain situations.",
        ),
        (
            4,
            "Your 2023 state and local income tax refund is more than your 2023 state and local \
             income tax deduction minus the amount you could have deducted as your 2023 state and \
             local general sales taxes.",
        ),
        (
            5,
            "You made your last payment of 2023 estimated state or local income tax in 2024.",
        ),
        (6, "You owed alternative minimum tax in 2023."),
        (
            7,
            "You couldn't use the full amount of credits you were entitled to in 2023 because the \
             total credits were more than the amount shown on your 2023 Form 1040 or 1040-SR, line \
             16.",
        ),
        (
            8,
            "You could be claimed as a dependent by someone else in 2023.",
        ),
        (
            9,
            "You received a refund because of a jointly filed state or local income tax return, \
             but you aren't filing a joint 2024 Form 1040 or 1040-SR with the same person.",
        ),
    ],
    lines: &[
        (
            1,
            "Enter the income tax refund from Form(s) 1099-G (or similar statement). But \
             don\u{2019}t enter more than the amount of your state and local income taxes shown on \
             your 2023 Schedule A, line 5d",
        ),
        (
            2,
            "Is the amount of state and local income taxes (or general sales taxes), real estate \
             taxes, and personal property taxes paid in 2023 (generally, this is the amount \
             reported on your 2023 Schedule A, line 5d) more than the amount on your 2023 Schedule \
             A, line 5e?",
        ),
        (3, "Is the amount on line 1 more than the amount on line 2?"),
        (
            4,
            "Enter your total itemized deductions from your 2023 Schedule A, line 17.",
        ),
        (
            5,
            "Enter the amount shown below for the filing status claimed on your 2023 Form 1040 or \
             1040-SR.",
        ),
        (6, "Check any boxes that apply.*"),
        (7, "Add lines 5 and 6"),
        (8, "Is the amount on line 7 less than the amount on line 4?"),
        (
            9,
            "Taxable part of your refund. Enter the smaller of line 3 or line 8 here and on \
             Schedule 1, line 1",
        ),
    ],
    line2_no: "No. Enter the amount from line 1 on line 3 and go to line 4.",
    line2_yes: "Yes. Subtract the amount on your 2023 Schedule A, line 5e, from the amount of \
                state and local income taxes (or general sales taxes), real estate taxes, and \
                personal property taxes paid in 2023 (generally, this is the amount reported on \
                your 2023 Schedule A, line 5d).",
    line3_yes: "Yes. Subtract line 2 from line 1.",
    stop: "None of your refund is taxable.",
    mfs_note: "Note. If the filing status on your 2023 Form 1040 or 1040-SR was married filing \
               separately and your spouse itemized deductions in 2023, skip lines 5 through 7, \
               enter the amount from line 4 on line 8, and go to line 9.",
    line5_bullets: &[
        Line5Bullet {
            statuses: &[FilingStatus::Single, FilingStatus::Mfs],
            amount: 13_850,
            printed: "\u{2022} Single or married filing separately\u{2014}$13,850",
        },
        Line5Bullet {
            statuses: &[FilingStatus::Mfj, FilingStatus::Qss],
            amount: 27_700,
            printed: "\u{2022} Married filing jointly or qualifying surviving \
                      spouse\u{2014}$27,700",
        },
        Line5Bullet {
            statuses: &[FilingStatus::HoH],
            amount: 20_800,
            printed: "\u{2022} Head of household\u{2014}$20,800",
        },
    ],
    line6_boxes: &[
        "You were born before January 2, 1959.",
        "You are blind.",
        "Spouse was born before January 2, 1959.",
        "Spouse is blind.",
    ],
    line6_no_boxes: "No boxes checked. Enter -0-.",
    line6_multiply: "Multiply the number of boxes checked by $1,500 ($1,850 if your 2023 filing \
                     status was single or head of household).",
    line6_per_box_married: 1_500,
    line6_per_box_unmarried: 1_850,
    line6_mfs_footnote: "*If your filing status is married filing separately, you can check the \
                         boxes for your spouse only if your spouse had no income, isn't filing a \
                         return, and can't be claimed as a dependent on another person's return.",
    line8_yes: "Yes. Subtract line 7 from line 4",
};

/// **TY2025** — `design/forms/extract/i1040gi--2025.txt`, worksheet at 42061-42157, line-1
/// instructions at 41878-41948. Figures a refund received in 2025 of tax paid in 2024.
pub const REVISION_2025: Revision = Revision {
    year: 2025,
    before_you_begin: "Be sure you have read the Exception in the instructions for this line to see \
                       if you can use this worksheet instead of Pub. 525 to figure if any of your \
                       refund is taxable.",
    tip: "if, in the year you paid the tax, you either (a) didn\u{2019}t itemize deductions, or (b) \
          elected to deduct state and local general sales taxes instead of state and local income \
          taxes.",
    exception_header: "Exception. See Itemized Deduction Recoveries in Pub. 525 instead of using \
                       the State and Local Income Tax Refund Worksheet in these instructions if \
                       any of the following applies.",
    exceptions: &[
        (
            1,
            "You received a refund in 2025 that is for a tax year other than 2024.",
        ),
        (
            2,
            "You received a refund other than an income tax refund, such as a general sales tax or \
             real property tax refund, in 2025 of an amount deducted or credit claimed in an \
             earlier year.",
        ),
        (
            3,
            "You had taxable income on your 2024 Form 1040 or 1040-SR, line 15, but no tax on your \
             Form 1040 or 1040-SR, line 16, because of the 0% tax rate on net capital gain and \
             qualified dividends in certain situations.",
        ),
        (
            4,
            "Your 2024 state and local income tax refund is more than your 2024 state and local \
             income tax deduction minus the amount you could have deducted as your 2024 state and \
             local general sales taxes.",
        ),
        (
            5,
            "You made your last payment of 2024 estimated state or local income tax in 2025.",
        ),
        (6, "You owed alternative minimum tax in 2024."),
        (
            7,
            "You couldn\u{2019}t use the full amount of credits you were entitled to in 2024 \
             because the total credits were more than the amount shown on your 2024 Form 1040 or \
             1040-SR, line 18.",
        ),
        (
            8,
            "You could be claimed as a dependent by someone else in 2024.",
        ),
        (
            9,
            "You received a refund because of a jointly filed state or local income tax return, \
             but you aren\u{2019}t filing a joint 2025 Form 1040 or 1040-SR with the same person.",
        ),
    ],
    lines: &[
        (
            1,
            "Enter the income tax refund from Form(s) 1099-G (or similar statement). But \
             don\u{2019}t enter more than the amount of your state and local income taxes shown on \
             your 2024 Schedule A, line 5d",
        ),
        (
            2,
            "Is the amount of state and local income taxes (or general sales taxes), real estate \
             taxes, and personal property taxes paid in 2024 (generally, this is the amount \
             reported on your 2024 Schedule A, line 5d) more than the amount on your 2024 Schedule \
             A, line 5e?",
        ),
        (3, "Is the amount on line 1 more than the amount on line 2?"),
        (
            4,
            "Enter your total itemized deductions from your 2024 Schedule A, line 17.",
        ),
        (
            5,
            "Enter the amount shown below for the filing status claimed on your 2024 Form 1040 or \
             1040-SR.",
        ),
        (6, "Check any boxes that apply.*"),
        (7, "Add lines 5 and 6"),
        (8, "Is the amount on line 7 less than the amount on line 4?"),
        (
            9,
            "Taxable part of your refund. Enter the smaller of line 3 or line 8 here and on \
             Schedule 1, line 1",
        ),
    ],
    line2_no: "No. Enter the amount from line 1 on line 3 and go to line 4.",
    line2_yes: "Yes. Subtract the amount on your 2024 Schedule A, line 5e, from the amount of \
                state and local income taxes (or general sales taxes), real estate taxes, and \
                personal property taxes paid in 2024 (generally, this is the amount reported on \
                your 2024 Schedule A, line 5d).",
    line3_yes: "Yes. Subtract line 2 from line 1.",
    stop: "None of your refund is taxable.",
    mfs_note: "Note. If the filing status on your 2024 Form 1040 or 1040-SR was married filing \
               separately and your spouse itemized deductions in 2024, skip lines 5 through 7, \
               enter the amount from line 4 on line 8, and go to line 9.",
    line5_bullets: &[
        Line5Bullet {
            statuses: &[FilingStatus::Single, FilingStatus::Mfs],
            amount: 14_600,
            printed: "\u{2022} Single or married filing separately\u{2014}$14,600",
        },
        Line5Bullet {
            statuses: &[FilingStatus::Mfj, FilingStatus::Qss],
            amount: 29_200,
            printed: "\u{2022} Married filing jointly or qualifying surviving \
                      spouse\u{2014}$29,200",
        },
        Line5Bullet {
            statuses: &[FilingStatus::HoH],
            amount: 21_900,
            printed: "\u{2022} Head of household\u{2014}$21,900",
        },
    ],
    line6_boxes: &[
        "You were born before January 2, 1960.",
        "You are blind.",
        "Spouse was born before January 2, 1960.",
        "Spouse is blind.",
    ],
    line6_no_boxes: "No boxes checked. Enter -0-.",
    line6_multiply: "Multiply the number of boxes checked by $1,550 ($1,950 if your 2024 filing \
                     status was single or head of household).",
    line6_per_box_married: 1_550,
    line6_per_box_unmarried: 1_950,
    line6_mfs_footnote: "*If your filing status is married filing separately, you can check the \
                         boxes for your spouse only if your spouse had no income, isn\u{2019}t \
                         filing a return, and can\u{2019}t be claimed as a dependent on another \
                         person\u{2019}s return.",
    line8_yes: "Yes. Subtract line 7 from line 4",
};

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The nine Pub. 525 Exception conditions.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// One of the **nine** conditions under which the worksheet may not be used at all — *"See Itemized
/// Deduction Recoveries in Pub. 525 instead of using the State and Local Income Tax Refund
/// Worksheet in these instructions if any of the following applies."*
///
/// ★★ **The variants carry no text.** Each revision prints its own wording (exception 7 cites 1040
/// line 18 in 2025 and line 16 in 2024), so the sentence lives on the [`Revision`] and the variant
/// is only the condition's identity. [`Revision::exception_text`] joins them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pub525Exception {
    /// 1 — the refund is for a tax year other than the immediately preceding one.
    E1RefundForAnotherYear,
    /// 2 — a refund of something other than income tax (a sales-tax or real-property-tax refund).
    E2NotAnIncomeTaxRefund,
    /// 3 — prior-year taxable income with no tax, because of the 0% rate on net capital gain and
    /// qualified dividends.
    E3ZeroRateOnPreferentialIncome,
    /// 4 — the refund exceeds the income-tax deduction minus the sales taxes that could have been
    /// deducted instead.
    E4RefundExceedsIncrementalDeduction,
    /// 5 — the last prior-year estimated state payment was made in the filing year.
    E5LastEstimatedPaymentInFilingYear,
    /// 6 — alternative minimum tax was owed in the prior year.
    E6OwedAmtInPriorYear,
    /// 7 — prior-year credits exceeded the tax they could offset.
    E7UnusableCredits,
    /// 8 — the filer could be claimed as someone else's dependent in the prior year.
    E8CouldBeClaimedAsDependent,
    /// 9 — the refund came from a jointly filed state return and this year's federal return is not
    /// joint with the same person.
    E9JointStateReturnNotJointNow,
}

impl Pub525Exception {
    /// Every condition, in the form's own order. ★ An `ALL` guarded by the `_`-free match in
    /// [`Self::number`] and by [`StateLocalRefundFacts::exception_that_applies`]: a tenth variant
    /// does not compile until both name it.
    pub const ALL: [Self; 9] = [
        Self::E1RefundForAnotherYear,
        Self::E2NotAnIncomeTaxRefund,
        Self::E3ZeroRateOnPreferentialIncome,
        Self::E4RefundExceedsIncrementalDeduction,
        Self::E5LastEstimatedPaymentInFilingYear,
        Self::E6OwedAmtInPriorYear,
        Self::E7UnusableCredits,
        Self::E8CouldBeClaimedAsDependent,
        Self::E9JointStateReturnNotJointNow,
    ];

    /// The number the form prints beside the condition.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::E1RefundForAnotherYear => 1,
            Self::E2NotAnIncomeTaxRefund => 2,
            Self::E3ZeroRateOnPreferentialIncome => 3,
            Self::E4RefundExceedsIncrementalDeduction => 4,
            Self::E5LastEstimatedPaymentInFilingYear => 5,
            Self::E6OwedAmtInPriorYear => 6,
            Self::E7UnusableCredits => 7,
            Self::E8CouldBeClaimedAsDependent => 8,
            Self::E9JointStateReturnNotJointNow => 9,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// What the filer must supply — the figures and declarations no other part of the return holds.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Line 6's four checkboxes, for the PRIOR year.
///
/// ★★ Deliberately NOT [`crate::tax::packet::AgedBlindBoxes`], which is THIS year's §63(f)
/// determination computed from this year's return. These four are the boxes as they stood on the
/// prior-year return: the aged label is a birth-date cutoff that moves every revision, and blindness
/// is a point-in-time status *"at the end of"* the prior year that btctax holds no record of. A
/// prior-year fact read off a prior-year return is the filer's to state.
///
/// ★ Every field is serde-REQUIRED (no `#[serde(default)]`): a TOML that omits one refuses to parse.
/// The whole block is `Option` on [`crate::tax::return_inputs::ReturnInputs`], so answered-ness
/// lives in the block's presence, and inside it no field can be silently `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorYearAgedBlindBoxes {
    /// Box 1 — *"You were born before January 2, &lt;cutoff&gt;."*
    pub taxpayer_aged: bool,
    /// Box 2 — *"You are blind."*
    pub taxpayer_blind: bool,
    /// Box 3 — *"Spouse was born before January 2, &lt;cutoff&gt;."*
    pub spouse_aged: bool,
    /// Box 4 — *"Spouse is blind."*
    pub spouse_blind: bool,
}

impl PriorYearAgedBlindBoxes {
    /// *"Multiply the number of boxes checked by …"* — the count, after the footnote's MFS gate.
    ///
    /// ★★★ **THE SPOUSE BOXES FAIL CLOSED FOR A PRIOR-YEAR MFS FILER**, exactly as
    /// [`crate::tax::packet::AgedBlindBoxes`] does for this year's: *"forgoing costs a deduction the
    /// filer can recover by answering while granting one they are not entitled to understates a
    /// signed return."* Here the direction is the same but one step removed — line 6 raises line 7,
    /// line 7 is subtracted from line 4, so an over-claimed box makes the TAXABLE PART SMALLER. The
    /// footnote's three conditions are one collected declaration
    /// ([`StateLocalRefundFacts::mfs_spouse_boxes_permitted`]); unless it is affirmed, a prior-year
    /// MFS filer's spouse boxes do not count.
    #[must_use]
    pub fn count(self, prior_status: FilingStatus, mfs_spouse_boxes_permitted: bool) -> u32 {
        let spouse_counts = prior_status != FilingStatus::Mfs || mfs_spouse_boxes_permitted;
        u32::from(self.taxpayer_aged)
            + u32::from(self.taxpayer_blind)
            + u32::from(spouse_counts && self.spouse_aged)
            + u32::from(spouse_counts && self.spouse_blind)
    }
}

/// Everything the §111(a) worksheet asks for that nothing else on the return holds: the nine
/// Exception conditions as the filer answered them, the prior-year figures lines 1, 2, 4, 5 and 6
/// read, and the refund no Form 1099-G reported.
///
/// ★★★ **COLLECTED, NOT CARRIED, and the reason is that the prior year is usually not in the
/// vault.** `open_next_year::seed`'s own rule is *"Never a carried amount"* — only the §1212(b)
/// carryforwards cross a year boundary, read from year N's frozen RETURN — and a test pins that it
/// carries no `schedule_a` at all. More decisively: a refund received in year *Y* is of tax paid in
/// *Y−1*, and the first year btctax files is by construction a year whose predecessor it did not
/// file. A carry-only design would leave exactly the filer FR-196 names still blocked. So these are
/// figures the filer reads off the prior-year return they are holding —
/// [`crate::tax::provenance::Source::FilerRecords`], the provenance Schedule A's own lines carry.
///
/// ★ **Nothing here forecloses the carry.** When the prior year IS in the vault, the same fields can
/// be seeded and stamped [`crate::tax::return_inputs::CarryProvenance::ComputedFromPriorReturn`];
/// [`Self::provenance`] is the sibling scalar that says which happened, so a carried figure and a
/// typed one are never the same bytes.
///
/// ★ Every field is serde-REQUIRED, per [`PriorYearAgedBlindBoxes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateLocalRefundFacts {
    /// **The TIP's limb (b)** — *"elected to deduct state and local general sales taxes instead of
    /// state and local income taxes"* in the year the tax was paid.
    ///
    /// ★★★ **The exit that was never asked.** `ReturnInputs::itemized_prior_year` asks the TIP's
    /// limb (a) and nothing asks limb (b), so a filer who itemized but elected SALES taxes hit the
    /// refusal with nothing taxable to compute. `true` ⇒ none of the refund is taxable and Schedule
    /// 1 line 1 is blank BY DECISION.
    pub prior_year_elected_sales_tax: bool,
    /// **Worksheet line 1's other half** — *"Enter the income tax refund from Form(s) 1099-G **(or
    /// similar statement)**"*, for the part of the refund NO Form 1099-G reported.
    ///
    /// ★★★ **Collected because the form asks for it and nothing on the return could answer.** The
    /// line-1 instructions are explicit: *"Report any taxable refund you received even if you didn't
    /// receive Form 1099-G"* (`i1040gi--2025.txt:41897-41898`), and that filer is reached through
    /// [`crate::tax::return_inputs::ReturnInputs::state_refund_without_1099g`] — a BOOLEAN with no
    /// amount beside it. So a `Yes` on that question declared a refund whose size was unstatable,
    /// and the only thing that hid it was the refusal.
    ///
    /// ★ The filer's-records analogue of
    /// [`ReturnInputs::schedule_b_filer_records`](crate::tax::return_inputs::ReturnInputs::schedule_b_filer_records):
    /// a `Yes` on a document-less income question OPENS a place to put the figure. A scalar rather
    /// than rows, because worksheet line 1 wants one total and no line of the return itemises it.
    ///
    /// ★★ **It ADDS to the transcribed Form 1099-G box 2 figures, never replaces them** — line 1 says
    /// *"Form(s)"*, plural, *"or similar statement"*, so a filer with one 1099-G and one refund
    /// cheque with no form behind it reports both. Zero is the lawful and overwhelmingly common
    /// value; a non-zero one beside `state_refund_without_1099g == Some(false)` is a contradiction
    /// `screen_inputs` should refuse, which is part of FR-196's wiring rather than of this module.
    pub refund_not_on_a_1099g: Usd,
    /// **The prior-year filing status** — *"the filing status claimed on your Y−1 Form 1040 or
    /// 1040-SR"*, which lines 5, 6 and the MFS Note all read. It is not this year's status: the
    /// worksheet exists because last year and this year are different returns.
    pub prior_year_filing_status: FilingStatus,
    /// **Line 1's cap** — *"the amount of your state and local income taxes shown on your Y−1
    /// Schedule A, line 5d"*. ★ Line **5d**, verbatim, which is the 5a+5b+5c total; see the module
    /// docs for why that reading is transcribed rather than adjudicated.
    pub prior_year_schedule_a_line5d: Usd,
    /// **Line 2's subtrahend** — *"the amount on your Y−1 Schedule A, line 5e"*, i.e. the
    /// §164(b)(6) limitation as that year's Schedule A applied it.
    pub prior_year_schedule_a_line5e: Usd,
    /// **Line 4** — *"your total itemized deductions from your Y−1 Schedule A, line 17"*.
    pub prior_year_schedule_a_line17: Usd,
    /// **The Note** — *"If the filing status on your Y−1 Form 1040 or 1040-SR was married filing
    /// separately and your spouse itemized deductions in Y−1, skip lines 5 through 7…"*. Read only
    /// when [`Self::prior_year_filing_status`] is [`FilingStatus::Mfs`].
    pub prior_year_mfs_spouse_itemized: bool,
    /// **Line 6's four boxes**, for the prior year.
    pub prior_year_aged_blind: PriorYearAgedBlindBoxes,
    /// **Line 6's footnote**, as one declaration — *"you can check the boxes for your spouse only
    /// if your spouse had no income, isn't filing a return, and can't be claimed as a dependent on
    /// another person's return."*
    ///
    /// ★ Phrased so `false` is the neutral answer and every omission forgoes, per
    /// `widening-an-exemption-is-never-the-safe-edit`: the three YES-conditions are conjoined into
    /// one affirmation, and anything short of it drops the spouse boxes.
    pub mfs_spouse_boxes_permitted: bool,
    /// **Exception 1** — *"You received a refund in Y that is for a tax year other than Y−1."*
    pub exception_refund_for_another_year: bool,
    /// **Exception 2** — *"You received a refund other than an income tax refund, such as a general
    /// sales tax or real property tax refund, in Y of an amount deducted or credit claimed in an
    /// earlier year."*
    pub exception_not_an_income_tax_refund: bool,
    /// **Exception 3** — *"You had taxable income on your Y−1 Form 1040 or 1040-SR, line 15, but no
    /// tax on your Form 1040 or 1040-SR, line 16, because of the 0% tax rate on net capital gain
    /// and qualified dividends in certain situations."*
    ///
    /// ★ Computable from a prior-year return btctax filed (lines 15 and 16 are both on
    /// `AbsoluteReturn`), and collected because that return usually does not exist. Naming the
    /// mechanism rather than the outcome, per `CLAUDE.md`.
    pub exception_zero_rate_on_preferential_income: bool,
    /// **Exception 4** — *"Your Y−1 state and local income tax refund is more than your Y−1 state
    /// and local income tax deduction minus the amount you could have deducted as your Y−1 state
    /// and local general sales taxes."*
    ///
    /// ★ Needs the §164(b)(5)(H) optional sales-tax TABLES, which btctax does not carry, so it is
    /// the filer's to answer for as long as that is true.
    pub exception_refund_exceeds_incremental_deduction: bool,
    /// **Exception 5** — *"You made your last payment of Y−1 estimated state or local income tax in
    /// Y."*
    pub exception_last_estimated_payment_in_filing_year: bool,
    /// **Exception 6** — *"You owed alternative minimum tax in Y−1."*
    ///
    /// ★ Also computable from a prior-year return btctax filed (Form 6251 is modelled), and
    /// collected for the same reason as exception 3.
    pub exception_owed_amt_in_prior_year: bool,
    /// **Exception 7** — *"You couldn't use the full amount of credits you were entitled to in Y−1
    /// because the total credits were more than the amount shown on your Y−1 Form 1040 or 1040-SR,
    /// line 18."* (The 2024 revision cites **line 16** — the 1040 renumbered.)
    pub exception_unusable_credits: bool,
    /// **Exception 8** — *"You could be claimed as a dependent by someone else in Y−1."*
    pub exception_could_be_claimed_as_dependent: bool,
    /// **Exception 9** — *"You received a refund because of a jointly filed state or local income
    /// tax return, but you aren't filing a joint Y Form 1040 or 1040-SR with the same person."*
    pub exception_joint_state_return_not_joint_now: bool,
    /// Whether these figures were typed by the filer or carried from a prior-year return btctax
    /// itself computed.
    ///
    /// ★ Present from the first commit rather than added later, for §G-23's *"stated zero"* reason:
    /// a `0` on line 5e is a real and common figure, and *"the filer said zero"* and *"btctax
    /// computed zero"* must not be the same bytes.
    pub provenance: crate::tax::return_inputs::CarryProvenance,
}

impl StateLocalRefundFacts {
    /// The FIRST Exception condition the filer affirmed, or `None` when the worksheet may be used.
    ///
    /// ★★★ **The `_`-free match over [`Pub525Exception::ALL`] is the point.** A tenth condition on a
    /// future revision adds a variant, and this match stops compiling until the new field is named —
    /// which is the one repair `CLAUDE.md` accepts for *"a list typed beside a set that grows"*.
    #[must_use]
    pub fn exception_that_applies(&self) -> Option<Pub525Exception> {
        Pub525Exception::ALL.into_iter().find(|e| match e {
            Pub525Exception::E1RefundForAnotherYear => self.exception_refund_for_another_year,
            Pub525Exception::E2NotAnIncomeTaxRefund => self.exception_not_an_income_tax_refund,
            Pub525Exception::E3ZeroRateOnPreferentialIncome => {
                self.exception_zero_rate_on_preferential_income
            }
            Pub525Exception::E4RefundExceedsIncrementalDeduction => {
                self.exception_refund_exceeds_incremental_deduction
            }
            Pub525Exception::E5LastEstimatedPaymentInFilingYear => {
                self.exception_last_estimated_payment_in_filing_year
            }
            Pub525Exception::E6OwedAmtInPriorYear => self.exception_owed_amt_in_prior_year,
            Pub525Exception::E7UnusableCredits => self.exception_unusable_credits,
            Pub525Exception::E8CouldBeClaimedAsDependent => {
                self.exception_could_be_claimed_as_dependent
            }
            Pub525Exception::E9JointStateReturnNotJointNow => {
                self.exception_joint_state_return_not_joint_now
            }
        })
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The worksheet.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Which STOP the worksheet reached. Both print *"None of your refund is taxable."*, and they are
/// distinguished because they say different things about WHY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopLine {
    /// Line 3 — the refund did not exceed the part of the prior year's taxes the §164(b)(6)
    /// limitation had already disallowed, so no deduction produced a benefit.
    Line3,
    /// Line 8 — the prior year's itemized total did not exceed the standard deduction that could
    /// have been taken instead, so itemizing produced no benefit.
    Line8,
}

/// Line 9, or the STOP that replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxablePart {
    /// *"None of your refund is taxable."* — Schedule 1 line 1 is **BLANK**, not zero.
    Nothing(StopLine),
    /// Line 9's amount, which goes on Schedule 1 line 1.
    Amount(Usd),
}

/// **State and Local Income Tax Refund Worksheet**, one field per numbered line.
///
/// A line the form SKIPS is `None`, never `-0-`: line 2 on its own *"No."* branch, and lines 5–7 on
/// the prior-year-MFS Note. A line after a STOP is `None` for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Worksheet {
    /// L1 — "Enter the income tax refund from Form(s) 1099-G (or similar statement). But don't
    ///       enter more than the amount of your state and local income taxes shown on your Y−1
    ///       Schedule A, line 5d"
    pub line1: Usd,
    /// L2 — "Is the amount of state and local income taxes (or general sales taxes), real estate
    ///       taxes, and personal property taxes paid in Y−1 (generally, this is the amount reported
    ///       on your Y−1 Schedule A, line 5d) more than the amount on your Y−1 Schedule A, line
    ///       5e?"
    ///
    /// `None` = the answer was *No*, which says *"Enter the amount from line 1 on line 3 and go to
    /// line 4"* — line 2's box stays empty and line 3's comparison is never made.
    pub line2: Option<Usd>,
    /// L3 — "Is the amount on line 1 more than the amount on line 2?"
    ///
    /// `None` = the STOP at line 3 was reached. On line 2's *No* branch the value is line 1, so it
    /// is `Some`, not `None`.
    pub line3: Option<Usd>,
    /// L4 — "Enter your total itemized deductions from your Y−1 Schedule A, line 17."
    ///
    /// `None` = the worksheet stopped at line 3 before reaching it.
    pub line4: Option<Usd>,
    /// L5 — "Enter the amount shown below for the filing status claimed on your Y−1 Form 1040 or
    ///       1040-SR."
    ///
    /// `None` = skipped, by the Note (prior-year MFS whose spouse itemized) or by an earlier STOP.
    pub line5: Option<Usd>,
    /// L6 — "Check any boxes that apply.*" / "Multiply the number of boxes checked by …"
    pub line6: Option<Usd>,
    /// L7 — "Add lines 5 and 6"
    pub line7: Option<Usd>,
    /// L8 — "Is the amount on line 7 less than the amount on line 4?"
    ///
    /// `None` = the STOP at line 8 was reached, or an earlier STOP was.
    pub line8: Option<Usd>,
    /// L9 — "Taxable part of your refund. Enter the smaller of line 3 or line 8 here and on
    ///       Schedule 1, line 1"
    ///
    /// `None` = a STOP was reached and Schedule 1 line 1 is blank.
    pub line9: Option<Usd>,
}

/// Which limb of the TIP made the refund non-taxable without the worksheet being worked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipLimb {
    /// *"(a) didn't itemize deductions"* — `ReturnInputs::itemized_prior_year == Some(false)`.
    DidNotItemize,
    /// *"(b) elected to deduct state and local general sales taxes instead of state and local
    /// income taxes"*.
    ElectedSalesTax,
}

/// What §111(a) decided for this return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The TIP answered it before the worksheet began. Schedule 1 line 1 is **blank by decision**.
    NotTaxableByTip(TipLimb),
    /// The worksheet was worked, line by line.
    Worksheet {
        /// Every line, as filled or skipped.
        sheet: Worksheet,
        /// Line 9, or the STOP that replaced it.
        part: TaxablePart,
    },
}

impl Decision {
    /// The amount for Schedule 1 line 1, or `None` when the line is correctly **blank**.
    ///
    /// ★ `None` and `Some(0)` are different answers and this type keeps them apart: the first is a
    /// line the form says not to fill in, the second a computed zero. A caller that collapses them
    /// re-creates §G-11.
    #[must_use]
    pub fn schedule_1_line1(&self) -> Option<Usd> {
        match self {
            Self::NotTaxableByTip(_) => None,
            Self::Worksheet { part, .. } => match part {
                TaxablePart::Nothing(_) => None,
                TaxablePart::Amount(a) => Some(*a),
            },
        }
    }
}

/// Why the worksheet could not be used, and btctax must refuse rather than print a figure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotUsable {
    /// No `i1040gi` revision is archived for this tax year, so lines 5 and 6's constants are
    /// unknown. ★ It does NOT fall back to a neighbouring year's figures.
    NoArchivedRevision {
        /// The tax year asked for.
        tax_year: i32,
    },
    /// The filer has not supplied the prior-year figures — `ReturnInputs::state_local_refund` is
    /// `None`. UNANSWERED, in the repo's own sense: nothing ever populated it.
    FactsNotCollected,
    /// One of the nine Exception conditions applies, so Pub. 525's *Itemized Deduction Recoveries*
    /// governs instead. btctax models no part of Pub. 525.
    Pub525Exception(Pub525Exception),
    /// Line 5's bullet list named no amount for the prior-year filing status. Unreachable given
    /// [`tests::every_filing_status_has_exactly_one_line5_bullet`]; carried as a value rather than a
    /// panic because this crate's compute core does not panic.
    NoLine5AmountForStatus(FilingStatus),
}

/// Work the worksheet — or say why it cannot be worked.
///
/// `itemized_prior_year` is the answer to the TIP's limb (a), which btctax already asks at return
/// level: `Some(false)` is the TIP's own exit and `Some(true)` sends the filer into the worksheet. A
/// `None` is an unanswered class-(A) declaration that `screen_inputs` refuses on before this is
/// reached; here it simply falls through to the facts, which refuse when absent.
///
/// `refund` is worksheet line 1's first sentence — *"the income tax refund from Form(s) 1099-G (or
/// similar statement)"* — before the line's own cap is applied.
pub fn figure(
    tax_year: i32,
    itemized_prior_year: Option<bool>,
    refund_on_forms_1099g: Usd,
    facts: Option<&StateLocalRefundFacts>,
) -> Result<Decision, NotUsable> {
    // ★ The TIP's limb (a), first, because it needs no revision and no prior-year figures: *"None of
    //   your refund is taxable if, in the year you paid the tax, you either (a) didn't itemize
    //   deductions…"*. This is the state `itemized_prior_year = Some(false)` already produces today
    //   — Schedule 1 line 1 blank BY DECISION — and it is unchanged.
    if itemized_prior_year == Some(false) {
        return Ok(Decision::NotTaxableByTip(TipLimb::DidNotItemize));
    }
    let facts = facts.ok_or(NotUsable::FactsNotCollected)?;
    // ★ The TIP's limb (b). Also revision-independent, and also an exit rather than a zero.
    if facts.prior_year_elected_sales_tax {
        return Ok(Decision::NotTaxableByTip(TipLimb::ElectedSalesTax));
    }
    // "Before you begin: Be sure you have read the Exception in the instructions for this line to
    //  see if you can use this worksheet instead of Pub. 525…"
    if let Some(e) = facts.exception_that_applies() {
        return Err(NotUsable::Pub525Exception(e));
    }
    let rev = revision_for(tax_year).ok_or(NotUsable::NoArchivedRevision { tax_year })?;

    // L1 — "Enter the income tax refund from Form(s) 1099-G (or similar statement). But don't enter
    //       more than the amount of your state and local income taxes shown on your Y−1 Schedule A,
    //       line 5d"
    //
    // ★★ **Both halves of the first sentence, added HERE rather than by the caller.** "Form(s)" is
    //    plural and "(or similar statement)" is the refund no form reported — so the caller supplies
    //    the transcribed Form 1099-G box 2 total and the block supplies the rest. A caller that had
    //    to remember to add them is a caller that can forget, in the understatement direction.
    let line1 = (refund_on_forms_1099g + facts.refund_not_on_a_1099g)
        .min(facts.prior_year_schedule_a_line5d);

    // L2 — "Is the amount of … taxes paid in Y−1 (generally, this is the amount reported on your
    //       Y−1 Schedule A, line 5d) more than the amount on your Y−1 Schedule A, line 5e?"
    //   No.  "Enter the amount from line 1 on line 3 and go to line 4."      ⇒ line 2 stays BLANK
    //   Yes. "Subtract the amount on your Y−1 Schedule A, line 5e, from …"
    let taxes_paid = facts.prior_year_schedule_a_line5d;
    let (line2, line3) = if taxes_paid > facts.prior_year_schedule_a_line5e {
        let l2 = taxes_paid - facts.prior_year_schedule_a_line5e;
        // L3 — "Is the amount on line 1 more than the amount on line 2?"
        //   No.  STOP "None of your refund is taxable."
        //   Yes. "Subtract line 2 from line 1."
        if line1 <= l2 {
            return Ok(Decision::Worksheet {
                sheet: Worksheet {
                    line1,
                    line2: Some(l2),
                    line3: None,
                    line4: None,
                    line5: None,
                    line6: None,
                    line7: None,
                    line8: None,
                    line9: None,
                },
                part: TaxablePart::Nothing(StopLine::Line3),
            });
        }
        (Some(l2), line1 - l2)
    } else {
        (None, line1)
    };

    // L4 — "Enter your total itemized deductions from your Y−1 Schedule A, line 17."
    let line4 = facts.prior_year_schedule_a_line17;

    // "Note. If the filing status on your Y−1 Form 1040 or 1040-SR was married filing separately and
    //  your spouse itemized deductions in Y−1, skip lines 5 through 7, enter the amount from line 4
    //  on line 8, and go to line 9."
    let mfs_skip =
        facts.prior_year_filing_status == FilingStatus::Mfs && facts.prior_year_mfs_spouse_itemized;
    let (line5, line6, line7, line8) = if mfs_skip {
        (None, None, None, line4)
    } else {
        let l5 = rev.line5_amount(facts.prior_year_filing_status).ok_or(
            NotUsable::NoLine5AmountForStatus(facts.prior_year_filing_status),
        )?;
        // L6 — "Multiply the number of boxes checked by $X ($Y if your Y−1 filing status was single
        //       or head of household)." / "No boxes checked. Enter -0-."
        let boxes = facts.prior_year_aged_blind.count(
            facts.prior_year_filing_status,
            facts.mfs_spouse_boxes_permitted,
        );
        let l6 = rev.line6_per_box(facts.prior_year_filing_status) * Usd::from(boxes);
        // L7 — "Add lines 5 and 6"
        let l7 = l5 + l6;
        // L8 — "Is the amount on line 7 less than the amount on line 4?"
        //   No.  STOP "None of your refund is taxable."
        //   Yes. "Subtract line 7 from line 4"
        if l7 >= line4 {
            return Ok(Decision::Worksheet {
                sheet: Worksheet {
                    line1,
                    line2,
                    line3: Some(line3),
                    line4: Some(line4),
                    line5: Some(l5),
                    line6: Some(l6),
                    line7: Some(l7),
                    line8: None,
                    line9: None,
                },
                part: TaxablePart::Nothing(StopLine::Line8),
            });
        }
        (Some(l5), Some(l6), Some(l7), line4 - l7)
    };
    // ★ No assertion that line 8 is positive: on the MFS-skip branch it IS line 4, which a filer
    //   whose prior-year Schedule A line 17 was zero would carry as zero. Line 9 is then a computed
    //   `-0-` that the form does say to enter — a stated zero, not a blank.

    // L9 — "Taxable part of your refund. Enter the smaller of line 3 or line 8 here and on
    //       Schedule 1, line 1"
    let line9 = line3.min(line8);
    Ok(Decision::Worksheet {
        sheet: Worksheet {
            line1,
            line2,
            line3: Some(line3),
            line4: Some(line4),
            line5,
            line6,
            line7,
            line8: Some(line8),
            line9: Some(line9),
        },
        part: TaxablePart::Amount(line9),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::collections::BTreeSet;

    /// The workspace root, from this crate's manifest directory (`return_refuse.rs`'s precedent).
    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("crates/btctax-core -> workspace root")
            .to_path_buf()
    }

    /// Collapse every run of whitespace to a single space.
    ///
    /// `pdftotext -layout` wraps a clause mid-sentence and lays the worksheet's answer-box column
    /// beside its label column, so nothing here can be compared line-for-line. Whitespace is the only
    /// thing normalised: every glyph, including the U+2019 apostrophes and U+2014 em dashes the IRS
    /// actually prints, must match.
    fn norm(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn extract_text(year: i32) -> String {
        let p = repo_root().join(format!("{EXTRACT_STEM}{year}.txt"));
        std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
    }

    /// The heading the worksheet block begins with, verbatim (U+2014 em dash).
    const WORKSHEET_TITLE: &str =
        "State and Local Income Tax Refund Worksheet\u{2014}Schedule 1, Line 1";

    /// Every archived `i1040gi` extract that actually prints this worksheet, as `(year, text)`.
    ///
    /// ★★ **Read out of the extract DIRECTORY, never from a list of years typed here** — the rule
    /// `return_refuse.rs::archived_w2_instructions` follows, and the repair for the T8 defect shape.
    /// Archiving `i1040gi--2026.txt` widens this set with no edit to this file, which is what makes
    /// [`every_archived_revision_is_transcribed`] an instrument rather than a restatement.
    fn archived_revisions() -> Vec<(i32, String)> {
        let dir = repo_root().join("design/forms/extract");
        let mut out: Vec<(i32, String)> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .map(|e| e.expect("a readable directory entry").path())
            .filter_map(|p| {
                let name = p.file_name()?.to_str()?.to_string();
                let year: i32 = name
                    .strip_prefix("i1040gi--")?
                    .strip_suffix(".txt")?
                    .parse()
                    .ok()?;
                let text = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
                text.contains(WORKSHEET_TITLE).then_some((year, text))
            })
            .collect();
        out.sort_by_key(|(y, _)| *y);
        assert!(
            out.len() >= 2,
            "only {} archived i1040gi revision(s) print the worksheet — the extract directory has \
             moved, and every check below would then be measuring nothing",
            out.len()
        );
        out
    }

    /// A standalone `N.` — a physical line that is nothing but a number and a full stop. That is the
    /// shape the worksheet's own LINE LABELS take in the `-layout` text, and the shape a
    /// cross-reference (*"…Schedule A, line 17."*) does not.
    ///
    /// ★ Getting this wrong was measured, not theorised: the first version of this check read tokens
    /// out of the whitespace-normalised blob, which made line 4's *"line 17."* and TY2024 exception
    /// 7's *"line 16."* look like worksheet lines 17 and 16.
    fn standalone_number(line: &str) -> Option<u8> {
        line.trim().strip_suffix('.')?.parse().ok()
    }

    /// The PHYSICAL lines of one extract's worksheet block.
    ///
    /// Starts at the heading. Ends after line 9's instruction plus the trailing blank lines and label
    /// numbers that belong to it — **derived**, by consuming everything after it that is blank or a
    /// standalone number, rather than by looking for a hand-typed `"9."`.
    fn worksheet_lines(text: &str) -> Vec<&str> {
        let all: Vec<&str> = text.lines().collect();
        let start = all
            .iter()
            .position(|l| l.trim() == WORKSHEET_TITLE)
            .expect("the extract prints the worksheet heading on its own line");
        let last = all
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, l)| l.trim_start().starts_with("Taxable part of your refund."))
            .map(|(i, _)| i)
            .expect("the block's last numbered instruction is line 9");
        let mut end = last;
        for (i, l) in all.iter().enumerate().skip(last + 1) {
            let t = l.trim();
            if t.is_empty() || standalone_number(t).is_some() {
                end = i;
            } else {
                break;
            }
        }
        all[start..=end].to_vec()
    }

    /// The worksheet block of one extract, normalised.
    fn worksheet_block(text: &str) -> String {
        norm(&worksheet_lines(text).join("\n"))
    }

    /// The PHYSICAL lines of one extract's Exception block: its header through its last condition.
    fn exception_lines(text: &str) -> Vec<&str> {
        let all: Vec<&str> = text.lines().collect();
        let start = all
            .iter()
            .position(|l| l.starts_with("Exception. See Itemized Deduction Recoveries"))
            .expect("the extract prints the Exception header");
        let end = all
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, l)| l.trim() == "Lines 2a and 2b")
            .map(|(i, _)| i)
            .expect("the Exception block ends where the alimony instructions begin");
        all[start..end].to_vec()
    }

    /// The Exception block of one extract, normalised.
    fn exception_block(text: &str) -> String {
        norm(&exception_lines(text).join("\n"))
    }

    /// The Exception condition numbers the form itself enumerates.
    ///
    /// ★ A different shape from the worksheet's: a condition number starts a physical line and is
    /// followed by the sentence (*"7. You couldn't use the full amount…"*), where a cross-reference
    /// (*"…1040-SR, line 16."*) ENDS one. So the two derivations cannot share a predicate, and
    /// pretending they could is how *"line 16."* was first counted as a tenth condition.
    fn exception_numbers(text: &str) -> BTreeSet<u8> {
        exception_lines(text)
            .iter()
            .filter_map(|l| {
                let (n, rest) = l.trim_start().split_once(". ")?;
                let n: u8 = n.parse().ok()?;
                rest.starts_with(|c: char| c.is_ascii_uppercase())
                    .then_some(n)
            })
            .collect()
    }

    /// How a quote's END is proved, so a TRUNCATION cannot pass as a verbatim match.
    ///
    /// ★★ A shortened citation is a SUBSTRING of the real one, which is exactly the Form 6251
    /// line-33 class; a containment check alone therefore passes every truncation.
    #[derive(Debug, Clone, Copy)]
    enum Terminator {
        /// A numbered instruction whose answer box is reached by dot leaders. `pdftotext` renders
        /// them as spaced periods, so the match must be followed by `" ."`.
        DotLeader,
        /// Prose ending at a full stop or question mark, followed by the next sentence, the next line
        /// number, or a bullet — so the next token must begin with a capital, a digit, `•`, `*` or
        /// `$`.
        ///
        /// ★ Weaker than [`Self::DotLeader`]: it would accept a truncation landing exactly on an
        /// internal sentence boundary. Stated rather than hidden, and the quotes that carry one are
        /// enumerated by [`no_sentence_end_quote_hides_an_internal_full_stop`].
        SentenceEnd,
    }

    /// Which region of the extract a quote must appear in.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Block {
        /// The worksheet block itself — a quote found only elsewhere in a 100-page document has not
        /// been located on the worksheet.
        Worksheet,
        /// The Exception's own numbered list.
        Exception,
        /// The line-1 instructions at large (the TIP, whose glyph interrupts its own sentence).
        WholeExtract,
    }

    /// Assert one quote is present, verbatim, and WHOLE.
    fn assert_quote(haystack: &str, label: &str, quote: &str, term: Terminator) {
        let needle = norm(quote);
        let at = haystack.find(&needle).unwrap_or_else(|| {
            panic!("{label}: not found verbatim in the extract.\n  wanted: {needle}")
        });
        let rest = &haystack[at + needle.len()..];
        match term {
            Terminator::DotLeader => assert!(
                rest.starts_with(" ."),
                "{label}: the match does not end where the form ends the instruction — it is \
                 followed by {:?}, not the dot leaders that run to the answer box. A TRUNCATED \
                 citation is a substring of the real one.",
                &rest[..rest.len().min(40)]
            ),
            Terminator::SentenceEnd => {
                let next = rest.trim_start();
                let ok = next.is_empty()
                    || next.starts_with(|c: char| {
                        c.is_ascii_uppercase()
                            || c.is_ascii_digit()
                            || c == '\u{2022}'
                            || c == '*'
                            || c == '$'
                    });
                assert!(
                    ok,
                    "{label}: the match ends mid-clause — the next token is {:?}, which begins \
                     lowercase, so the quote is TRUNCATED.",
                    &next[..next.len().min(40)]
                );
            }
        }
    }

    /// Every quote a revision claims is verbatim, with the block it lives in and how its end is
    /// proved.
    ///
    /// ★ **`_`-free destructuring of [`Revision`]**: a field added to the struct does not compile
    /// until it is either quoted here or explicitly bound and excused — the half a containment check
    /// can never see. The pattern `capital_loss_carryover_check` established for the same job, moved
    /// into the crate that owns the data.
    fn quotes_of(rev: &Revision) -> Vec<(String, &'static str, Terminator, Block)> {
        let Revision {
            // Not quotes: the year keys the extract, and the two numeric constants are cross-checked
            // against `line6_multiply` by `every_printed_amount_agrees_with_the_figure_beside_it`.
            year: _,
            line6_per_box_married: _,
            line6_per_box_unmarried: _,
            before_you_begin,
            tip,
            exception_header,
            exceptions,
            lines,
            line2_no,
            line2_yes,
            line3_yes,
            stop,
            mfs_note,
            line5_bullets,
            line6_boxes,
            line6_no_boxes,
            line6_multiply,
            line6_mfs_footnote,
            line8_yes,
        } = rev;
        let mut v: Vec<(String, &'static str, Terminator, Block)> = Vec::new();
        for (n, q) in *lines {
            // Lines 1, 7 and 9 run dot leaders to an answer box on their own row; 2, 3, 4, 5, 6 and 8
            // are questions or headings whose box sits in the column beside them.
            let term = match n {
                1 | 7 | 9 => Terminator::DotLeader,
                _ => Terminator::SentenceEnd,
            };
            v.push((format!("line {n}"), *q, term, Block::Worksheet));
        }
        for (label, q) in [
            ("before you begin", *before_you_begin),
            ("line 2 — the No branch", *line2_no),
            ("line 2 — the Yes branch", *line2_yes),
            ("line 3 — the Yes branch", *line3_yes),
            ("the STOP text", *stop),
            ("the MFS Note", *mfs_note),
            ("line 6 — no boxes checked", *line6_no_boxes),
            ("line 6 — multiply", *line6_multiply),
            ("line 6 — the MFS footnote", *line6_mfs_footnote),
        ] {
            v.push((
                label.to_string(),
                q,
                Terminator::SentenceEnd,
                Block::Worksheet,
            ));
        }
        for b in *line5_bullets {
            v.push((
                format!("line 5 bullet {:?}", b.statuses),
                b.printed,
                Terminator::SentenceEnd,
                Block::Worksheet,
            ));
        }
        for (i, b) in line6_boxes.iter().enumerate() {
            v.push((
                format!("line 6 box {}", i + 1),
                b,
                Terminator::SentenceEnd,
                Block::Worksheet,
            ));
        }
        v.push((
            "line 8 — the Yes branch".to_string(),
            line8_yes,
            Terminator::DotLeader,
            Block::Worksheet,
        ));
        // The TIP and the Exception live in the LINE-1 INSTRUCTIONS, not in the worksheet block.
        v.push((
            "the TIP".to_string(),
            tip,
            Terminator::SentenceEnd,
            Block::WholeExtract,
        ));
        v.push((
            "the Exception header".to_string(),
            exception_header,
            Terminator::SentenceEnd,
            Block::Exception,
        ));
        for (n, q) in *exceptions {
            v.push((
                format!("exception {n}"),
                *q,
                Terminator::SentenceEnd,
                Block::Exception,
            ));
        }
        v
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // 1. VERBATIM and WHOLE.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ Every quote in every revision appears in that revision's extract, verbatim and whole.
    #[test]
    fn every_quote_is_verbatim_and_whole() {
        for rev in REVISIONS {
            let text = extract_text(rev.year);
            let ws = worksheet_block(&text);
            let ex = exception_block(&text);
            let whole = norm(&text);
            for (label, quote, term, block) in quotes_of(rev) {
                let haystack = match block {
                    Block::Worksheet => &ws,
                    Block::Exception => &ex,
                    Block::WholeExtract => &whole,
                };
                assert_quote(haystack, &format!("TY{} {label}", rev.year), quote, term);
            }
        }
    }

    /// ★ The stated limit of [`Terminator::SentenceEnd`], MEASURED rather than asserted: a quote with
    /// an internal full stop could be truncated there and still pass. Naming the ones that carry one
    /// is what keeps the blind spot reviewable.
    #[test]
    fn no_sentence_end_quote_hides_an_internal_full_stop() {
        // ★ Each is a two-sentence instruction the form prints as one block: line 2's and line 3's
        //   branches (a "No."/"Yes." label plus its instruction), the "Note." label, "No boxes
        //   checked. Enter -0-.", and "Before you begin"'s "Pub. 525" abbreviation. A quote NOT on
        //   this list must be single-sentence, or the weaker terminator protects less than it looks.
        let expected: BTreeSet<&str> = [
            "the MFS Note",
            "line 2 — the No branch",
            "line 2 — the Yes branch",
            "line 3 — the Yes branch",
            "line 6 — no boxes checked",
            "line 8 — the Yes branch",
            "before you begin",
            "the TIP",
            "the Exception header",
        ]
        .into_iter()
        .collect();
        let mut unexpected: Vec<String> = Vec::new();
        for rev in REVISIONS {
            for (label, quote, term, _) in quotes_of(rev) {
                if !matches!(term, Terminator::SentenceEnd) {
                    continue;
                }
                let body = norm(quote);
                let trimmed = body.trim_end_matches(['.', '?']);
                if trimmed.contains(". ") && !expected.contains(label.as_str()) {
                    unexpected.push(format!("TY{} {label}", rev.year));
                }
            }
        }
        assert!(
            unexpected.is_empty(),
            "these quotes span two sentences and carry only the WEAK terminator, so a truncation at \
             the internal full stop would pass: {unexpected:?}"
        );
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // 2. COMPLETE — every set is read OFF THE FORM.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// The worksheet LINE numbers the form itself enumerates — every physical line that is nothing
    /// but a standalone `N.` label.
    fn worksheet_numbers(text: &str) -> BTreeSet<u8> {
        worksheet_lines(text)
            .iter()
            .filter_map(|l| standalone_number(l))
            .collect()
    }

    /// ★★★ The set of worksheet line numbers transcribed is exactly the set the extract's own
    /// worksheet block enumerates — never a `1..=N` range and never a hand list, the two ways this
    /// repo has already got the same check wrong once each.
    #[test]
    fn the_line_numbers_are_read_off_the_form() {
        for rev in REVISIONS {
            let got: BTreeSet<u8> = rev.lines.iter().map(|(n, _)| *n).collect();
            assert_line_numbers(rev.year, &got);
        }
    }

    /// The line-number comparison, ONE function, so a kill can hand it a gutted set and watch the
    /// real assertion fire rather than merely observing that two sets differ.
    fn assert_line_numbers(year: i32, transcribed: &BTreeSet<u8>) {
        let want = worksheet_numbers(&extract_text(year));
        assert_eq!(
            *transcribed, want,
            "TY{year}: the transcribed line numbers are not the ones the form prints"
        );
        assert_eq!(want.len(), 9, "TY{year}: the worksheet has nine lines");
    }

    /// ★★★ The nine Exception conditions are read off the form too. A tenth condition on a future
    /// revision reds here BEFORE it can be silently un-modelled — which matters more than the line
    /// set, because an un-modelled exception is a worksheet run that should have been a refusal.
    #[test]
    fn the_exception_numbers_are_read_off_the_form() {
        for rev in REVISIONS {
            let got: BTreeSet<u8> = rev.exceptions.iter().map(|(n, _)| *n).collect();
            let modelled: BTreeSet<u8> = Pub525Exception::ALL.iter().map(|e| e.number()).collect();
            assert_exception_numbers(rev.year, &got, &modelled);
        }
    }

    /// The Exception comparison, ONE function, for [`assert_line_numbers`]'s reason.
    fn assert_exception_numbers(year: i32, transcribed: &BTreeSet<u8>, modelled: &BTreeSet<u8>) {
        let want = exception_numbers(&extract_text(year));
        assert_eq!(
            *transcribed, want,
            "TY{year}: the transcribed Exception conditions are not the ones the form prints"
        );
        assert_eq!(
            *modelled, want,
            "TY{year}: `Pub525Exception::ALL` and the form's own list have diverged"
        );
    }

    /// ★★★ **Every archived revision is transcribed** — the T8 repair. The expected set comes from
    /// the extract DIRECTORY, so `xtask forms-fetch` landing `i1040gi--2026.txt` reds this test
    /// rather than silently leaving TY2026 on [`NotUsable::NoArchivedRevision`] with nobody told.
    #[test]
    fn every_archived_revision_is_transcribed() {
        let transcribed: BTreeSet<i32> = REVISIONS.iter().map(|r| r.year).collect();
        assert_revisions_transcribed(&transcribed);
    }

    /// The revision-set comparison, ONE function, for [`assert_line_numbers`]'s reason.
    fn assert_revisions_transcribed(transcribed: &BTreeSet<i32>) {
        let archived: BTreeSet<i32> = archived_revisions().into_iter().map(|(y, _)| y).collect();
        assert_eq!(
            *transcribed, archived,
            "the archived i1040gi revisions that print this worksheet and the transcribed ones have \
             diverged. A NEW archive must be transcribed (lines 5 and 6 carry that year's constants \
             and nothing else can supply them); a REMOVED one must be deleted here."
        );
    }

    /// Every revision's `prior_year` is the year its own text names, read off the text.
    #[test]
    fn each_revision_names_its_own_prior_year() {
        for rev in REVISIONS {
            assert_eq!(rev.prior_year(), rev.year - 1);
            let l4 = rev
                .lines
                .iter()
                .find(|(n, _)| *n == 4)
                .expect("line 4 exists")
                .1;
            assert!(
                l4.contains(&format!("{} Schedule A, line 17", rev.prior_year())),
                "TY{}: line 4 must name the PRIOR year's Schedule A, and says {l4:?}",
                rev.year
            );
            assert_eq!(
                rev.source_extract(),
                format!("{EXTRACT_STEM}{}.txt", rev.year)
            );
        }
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // 3. The printed constants agree with the figures beside them.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// Format a whole-dollar amount the way the form prints it.
    fn printed_dollars(n: u32) -> String {
        let s = n.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        format!("${out}")
    }

    /// ★★★ Every numeric constant is checked against the sentence that prints it, in the order the
    /// sentence prints them. So the pair `amount` / `printed` cannot drift, and the line-6 pair cannot
    /// be swapped — the single most likely typo here, and one that moves money.
    #[test]
    fn every_printed_amount_agrees_with_the_figure_beside_it() {
        for rev in REVISIONS {
            for b in rev.line5_bullets {
                assert!(
                    b.printed.ends_with(&printed_dollars(b.amount)),
                    "TY{}: bullet {:?} does not print {}",
                    rev.year,
                    b.printed,
                    printed_dollars(b.amount)
                );
            }
            // "Multiply the number of boxes checked by $1,550 ($1,950 if your 2024 filing status was
            //  single or head of household)." — married FIRST, unmarried in the parenthesis.
            let m = printed_dollars(rev.line6_per_box_married);
            let u = printed_dollars(rev.line6_per_box_unmarried);
            let im = rev.line6_multiply.find(&m).unwrap_or_else(|| {
                panic!(
                    "TY{}: line 6 does not print the married amount {m}",
                    rev.year
                )
            });
            let iu = rev.line6_multiply.find(&u).unwrap_or_else(|| {
                panic!(
                    "TY{}: line 6 does not print the unmarried amount {u}",
                    rev.year
                )
            });
            assert!(
                im < iu,
                "TY{}: the married amount must come FIRST — the parenthesis is the single / head of \
                 household one. Swapping them changes line 6 for every filer.",
                rev.year
            );
            assert!(
                rev.line6_multiply.contains("single or head of household"),
                "TY{}: line 6's parenthesis must name the statuses that take the larger amount",
                rev.year
            );
        }
    }

    /// ★★★ Every [`FilingStatus`] is covered by exactly one line-5 bullet, so
    /// [`Revision::line5_amount`] can never return `None` and no status can be silently dropped when
    /// a bullet's wording changes.
    #[test]
    fn every_filing_status_has_exactly_one_line5_bullet() {
        for rev in REVISIONS {
            for st in [
                FilingStatus::Single,
                FilingStatus::Mfj,
                FilingStatus::Mfs,
                FilingStatus::HoH,
                FilingStatus::Qss,
            ] {
                let hits = rev
                    .line5_bullets
                    .iter()
                    .filter(|b| b.statuses.contains(&st))
                    .count();
                assert_eq!(
                    hits, 1,
                    "TY{}: {st:?} is named by {hits} line-5 bullets, not exactly one",
                    rev.year
                );
                assert!(rev.line5_amount(st).is_some());
            }
            // ★ Qss takes the MARRIED per-box amount; the parenthetical names single and HoH only.
            assert_eq!(
                rev.line6_per_box(FilingStatus::Qss),
                Usd::from(rev.line6_per_box_married)
            );
            assert_eq!(
                rev.line6_per_box(FilingStatus::HoH),
                Usd::from(rev.line6_per_box_unmarried)
            );
        }
    }

    /// The figures the 2025 revision prints ARE the TY2024 statutory ones this repo already ships —
    /// an independent witness that the transcription is not merely self-consistent.
    #[test]
    fn the_2025_revisions_printed_figures_are_the_ty2024_statutory_ones() {
        assert_eq!(
            REVISION_2025.line5_amount(FilingStatus::Single),
            Some(dec!(14600))
        );
        assert_eq!(
            REVISION_2025.line5_amount(FilingStatus::Mfj),
            Some(dec!(29200))
        );
        assert_eq!(
            REVISION_2025.line5_amount(FilingStatus::HoH),
            Some(dec!(21900))
        );
        // §63(f), Rev. Proc. 2023-34 §3.15(3) — the figures `tax_tables.rs::ty2024_full_return`
        // carries as `std_aged_blind_married` / `std_aged_blind_unmarried`.
        assert_eq!(REVISION_2025.line6_per_box(FilingStatus::Mfj), dec!(1550));
        assert_eq!(
            REVISION_2025.line6_per_box(FilingStatus::Single),
            dec!(1950)
        );
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // 4. B1 KILLS — each check observed RED on the exact defect it exists to catch.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// ★★★ **KILL — a PARAPHRASE reds.**
    #[test]
    #[should_panic(expected = "not found verbatim")]
    fn a_paraphrase_is_rejected() {
        let ws = worksheet_block(&extract_text(2025));
        assert_quote(
            &ws,
            "line 4, paraphrased",
            "Enter your total itemised deductions from your 2024 Schedule A, line 17.",
            Terminator::SentenceEnd,
        );
    }

    /// ★★★ **KILL — a TRUNCATION reds**, which a containment check alone cannot catch because a
    /// shortened citation is a substring of the real one. This is the Form 6251 line-33 class.
    #[test]
    #[should_panic(expected = "does not end where the form ends the instruction")]
    fn a_truncation_is_rejected() {
        let ws = worksheet_block(&extract_text(2025));
        assert_quote(
            &ws,
            "line 1, truncated after its first sentence",
            "Enter the income tax refund from Form(s) 1099-G (or similar statement).",
            Terminator::DotLeader,
        );
    }

    /// ★★★ **KILL — a WRONG CROSS-REFERENCE reds.** The planted defect is the real one: reading a line
    /// number off a rendered page as *"line 12"* where the form says *"line 17"*.
    #[test]
    #[should_panic(expected = "not found verbatim")]
    fn a_wrong_line_cross_reference_is_rejected() {
        let ws = worksheet_block(&extract_text(2025));
        assert_quote(
            &ws,
            "line 4 with the wrong cross-reference",
            "Enter your total itemized deductions from your 2024 Schedule A, line 12.",
            Terminator::SentenceEnd,
        );
    }

    /// ★★★ **KILL — a quote from the WRONG REVISION reds.** The 2024 revision's line 4 names the 2023
    /// Schedule A, and looking for it in the 2025 extract's worksheet block must fail — which is what
    /// makes the per-revision table load-bearing rather than decorative.
    #[test]
    #[should_panic(expected = "not found verbatim")]
    fn a_quote_from_the_wrong_revision_is_rejected() {
        let ws = worksheet_block(&extract_text(2025));
        let l4 = REVISION_2024
            .lines
            .iter()
            .find(|(n, _)| *n == 4)
            .expect("line 4 exists")
            .1;
        assert_quote(
            &ws,
            "TY2024 line 4 against the TY2025 extract",
            l4,
            Terminator::SentenceEnd,
        );
    }

    /// ★★★ **KILL — a DROPPED LINE reds.** Nine faithful whole quotes are still wrong if the form has
    /// nine lines and only eight were typed.
    #[test]
    #[should_panic(expected = "the transcribed line numbers are not the ones the form prints")]
    fn dropping_a_line_reds_the_completeness_check() {
        let gutted: BTreeSet<u8> = REVISION_2025
            .lines
            .iter()
            .map(|(n, _)| *n)
            .filter(|n| *n != 9)
            .collect();
        assert_line_numbers(2025, &gutted);
    }

    /// ★★★ **KILL — a DROPPED EXCEPTION reds**, and it is the more dangerous omission: an un-modelled
    /// exception turns a required Pub. 525 refusal into a computed figure.
    #[test]
    #[should_panic(expected = "`Pub525Exception::ALL` and the form's own list have diverged")]
    fn dropping_an_exception_reds_the_completeness_check() {
        let transcribed: BTreeSet<u8> = REVISION_2025.exceptions.iter().map(|(n, _)| *n).collect();
        let gutted: BTreeSet<u8> = Pub525Exception::ALL
            .iter()
            .map(|e| e.number())
            .filter(|n| *n != 6)
            .collect();
        assert_exception_numbers(2025, &transcribed, &gutted);
    }

    /// ★★★ **KILL — an UNTRANSCRIBED archived revision reds.** The instrument that exists because a
    /// list is correct on the day it is typed and the set grows underneath it.
    #[test]
    #[should_panic(expected = "and the transcribed ones have diverged")]
    fn an_untranscribed_archived_revision_reds() {
        let mut gutted: BTreeSet<i32> = REVISIONS.iter().map(|r| r.year).collect();
        gutted.remove(&2025);
        assert_revisions_transcribed(&gutted);
    }

    /// ★★★ **KILL — swapping line 6's two per-box amounts reds.** A one-token typo that moves money
    /// on every non-MFJ return.
    #[test]
    #[should_panic(expected = "the married amount must come FIRST")]
    fn swapping_line6s_per_box_amounts_reds() {
        let mut rev = REVISION_2025;
        rev.line6_per_box_married = 1_950;
        rev.line6_per_box_unmarried = 1_550;
        let m = printed_dollars(rev.line6_per_box_married);
        let u = printed_dollars(rev.line6_per_box_unmarried);
        let im = rev.line6_multiply.find(&m).expect("prints $1,950");
        let iu = rev.line6_multiply.find(&u).expect("prints $1,550");
        assert!(
            im < iu,
            "TY{}: the married amount must come FIRST — the parenthesis is the single / head of \
             household one. Swapping them changes line 6 for every filer.",
            rev.year
        );
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // 5. B1 VECTORS — the three the brief names, plus every exit.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// A filer who itemized, elected income taxes, and hit none of the nine exceptions.
    fn itemizer(
        status: FilingStatus,
        line5d: Usd,
        line5e: Usd,
        line17: Usd,
    ) -> StateLocalRefundFacts {
        StateLocalRefundFacts {
            prior_year_elected_sales_tax: false,
            refund_not_on_a_1099g: Usd::ZERO,
            prior_year_filing_status: status,
            prior_year_schedule_a_line5d: line5d,
            prior_year_schedule_a_line5e: line5e,
            prior_year_schedule_a_line17: line17,
            prior_year_mfs_spouse_itemized: false,
            prior_year_aged_blind: PriorYearAgedBlindBoxes {
                taxpayer_aged: false,
                taxpayer_blind: false,
                spouse_aged: false,
                spouse_blind: false,
            },
            mfs_spouse_boxes_permitted: false,
            exception_refund_for_another_year: false,
            exception_not_an_income_tax_refund: false,
            exception_zero_rate_on_preferential_income: false,
            exception_refund_exceeds_incremental_deduction: false,
            exception_last_estimated_payment_in_filing_year: false,
            exception_owed_amt_in_prior_year: false,
            exception_unusable_credits: false,
            exception_could_be_claimed_as_dependent: false,
            exception_joint_state_return_not_joint_now: false,
            provenance: crate::tax::return_inputs::CarryProvenance::User,
        }
    }

    /// ★★★ **VECTOR 1 — THE REFUND IS FULLY TAXABLE.** Single, TY2025 return, $900 refund of TY2024
    /// state income tax. TY2024 Schedule A line 5d $8,000 (under the $10,000 §164(b)(6) cap, so 5e is
    /// the same $8,000) and line 17 $20,000, well clear of the $14,600 standard deduction.
    ///
    /// Every dollar of the tax deducted produced a benefit, so every dollar of the refund is income.
    #[test]
    fn vector1_the_refund_is_fully_taxable() {
        let facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("the worksheet was worked, not exited by the TIP: {d:?}")
        };
        assert_eq!(sheet.line1, dec!(900), "the $8,000 cap does not bite");
        assert_eq!(
            sheet.line2, None,
            "5d is NOT more than 5e ⇒ line 2 stays BLANK"
        );
        assert_eq!(sheet.line3, Some(dec!(900)), "line 3 takes line 1 whole");
        assert_eq!(sheet.line4, Some(dec!(20000)));
        assert_eq!(sheet.line5, Some(dec!(14600)));
        assert_eq!(sheet.line6, Some(Usd::ZERO), "no boxes checked ⇒ -0-");
        assert_eq!(sheet.line7, Some(dec!(14600)));
        assert_eq!(sheet.line8, Some(dec!(5400)), "20,000 − 14,600");
        assert_eq!(sheet.line9, Some(dec!(900)), "the smaller of 900 and 5,400");
        assert_eq!(part, TaxablePart::Amount(dec!(900)));
        assert_eq!(d.schedule_1_line1(), Some(dec!(900)));
    }

    /// ★★★ **VECTOR 2 — THE TAX BENEFIT WAS ONLY PARTIAL.** MFJ, TY2025 return, $3,000 refund. The
    /// TY2024 itemized total was $30,000 against a $29,200 standard deduction, so itemizing bought
    /// only **$800** of benefit — and line 9's *"smaller of line 3 or line 8"* is what limits the
    /// income to it. $2,200 of the refund is NOT income.
    ///
    /// ★ This is the vector a closed form gets wrong: `taxable = refund` is right on vector 1 and
    /// overstates here by $2,200, while `taxable = 0` understates by $800.
    #[test]
    fn vector2_the_tax_benefit_was_only_partial() {
        let facts = itemizer(FilingStatus::Mfj, dec!(9000), dec!(9000), dec!(30000));
        let d = figure(2025, Some(true), dec!(3000), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("the worksheet was worked: {d:?}")
        };
        assert_eq!(sheet.line3, Some(dec!(3000)));
        assert_eq!(sheet.line7, Some(dec!(29200)));
        assert_eq!(
            sheet.line8,
            Some(dec!(800)),
            "30,000 − 29,200 — the WHOLE benefit"
        );
        assert_eq!(
            sheet.line9,
            Some(dec!(800)),
            "line 9 is the smaller of line 3 ($3,000) and line 8 ($800)"
        );
        assert_eq!(part, TaxablePart::Amount(dec!(800)));
        assert_eq!(d.schedule_1_line1(), Some(dec!(800)));
    }

    /// ★★★ **VECTOR 3 — THE FILER TOOK THE STANDARD DEDUCTION, SO NONE OF IT IS TAXABLE, AND THE LINE
    /// COMES OUT BLANK RATHER THAN ZERO.** The refusal's own stated exit: *"answer 'no' if you did not
    /// itemize in the year you paid the tax — then none of the refund is taxable and Schedule 1 line 1
    /// is correctly blank."*
    ///
    /// ★ `schedule_1_line1() == None`, not `Some(0)`. A hardcoded zero and a computed zero are
    /// indistinguishable on the printed page and are not the same thing; here neither is right,
    /// because the correct entry is NO entry.
    #[test]
    fn vector3_the_standard_deduction_year_leaves_the_line_blank_not_zero() {
        // No facts needed at all — limb (a) of the TIP decides before the worksheet begins.
        let d = figure(2025, Some(false), dec!(900), None).expect("the TIP decides");
        assert_eq!(d, Decision::NotTaxableByTip(TipLimb::DidNotItemize));
        assert_eq!(
            d.schedule_1_line1(),
            None,
            "Schedule 1 line 1 is BLANK by decision — not a computed zero"
        );
        // ★ And the same holds when the facts ARE present: the ANSWER decides, not the presence of a
        //   block. A filer who typed the prior-year figures and then answered "no" must not be routed
        //   into the worksheet by the block's mere existence.
        let facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        let d2 = figure(2025, Some(false), dec!(900), Some(&facts)).expect("the TIP still decides");
        assert_eq!(d2, Decision::NotTaxableByTip(TipLimb::DidNotItemize));
        assert_eq!(d2.schedule_1_line1(), None);
    }

    /// ★★★ **VECTOR 3b — the TIP's OTHER limb, which nothing in btctax asked before now.** An
    /// itemizer who ELECTED SALES TAXES deducted no income tax at all, so §111(a) makes none of the
    /// refund income — and today that filer is refused. The line is blank, not zero.
    #[test]
    fn vector3b_the_sales_tax_election_year_also_leaves_the_line_blank() {
        let mut facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        facts.prior_year_elected_sales_tax = true;
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the TIP decides");
        assert_eq!(d, Decision::NotTaxableByTip(TipLimb::ElectedSalesTax));
        assert_eq!(d.schedule_1_line1(), None);
    }

    /// ★★ **The line-3 STOP** — the §164(b)(6) limitation had already disallowed more than the whole
    /// refund, so none of the tax deducted produced a benefit. Line 9 is not reached and the line is
    /// blank.
    #[test]
    fn the_line3_stop_is_reached_when_the_salt_cap_swallowed_the_refund() {
        // TY2024 line 5d $18,000 against a $10,000 line 5e ⇒ line 2 = $8,000, and a $900 refund is
        // not more than that.
        let facts = itemizer(FilingStatus::Single, dec!(18000), dec!(10000), dec!(20000));
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("the worksheet was worked: {d:?}")
        };
        assert_eq!(sheet.line2, Some(dec!(8000)));
        assert_eq!(part, TaxablePart::Nothing(StopLine::Line3));
        assert_eq!(
            sheet.line3, None,
            "the STOP means line 3 is never filled in"
        );
        assert_eq!(sheet.line4, None, "…and nothing after it is");
        assert_eq!(d.schedule_1_line1(), None);
    }

    /// ★★ **The line-8 STOP** — the prior year's itemized total did not exceed the standard deduction
    /// that could have been taken, so itemizing bought nothing.
    #[test]
    fn the_line8_stop_is_reached_when_itemizing_bought_nothing() {
        let facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(14000));
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("the worksheet was worked: {d:?}")
        };
        assert_eq!(sheet.line7, Some(dec!(14600)));
        assert_eq!(part, TaxablePart::Nothing(StopLine::Line8));
        assert_eq!(sheet.line8, None);
        assert_eq!(sheet.line9, None);
        assert_eq!(d.schedule_1_line1(), None);
    }

    /// ★★ **Line 1's cap binds.** A refund larger than the prior year's Schedule A line 5d is limited
    /// to it — *"But don't enter more than the amount of your state and local income taxes shown on
    /// your Y−1 Schedule A, line 5d."*
    #[test]
    fn line1_is_capped_at_the_prior_years_schedule_a_line5d() {
        let facts = itemizer(FilingStatus::Single, dec!(500), dec!(500), dec!(20000));
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, .. } = d else {
            panic!("{d:?}")
        };
        assert_eq!(
            sheet.line1,
            dec!(500),
            "capped at line 5d, not the $900 refund"
        );
        assert_eq!(sheet.line9, Some(dec!(500)));
    }

    /// ★★★ **A refund NO Form 1099-G reported reaches line 1 too** — *"Report any taxable refund you
    /// received even if you didn't receive Form 1099-G"*. Before this block there was a boolean
    /// declaring such a refund and nowhere to state its size, so the figure could only ever have been
    /// zero: an understatement laundered as a lawful blank, invisible behind the refusal.
    #[test]
    fn a_refund_with_no_1099g_behind_it_still_reaches_line1() {
        let mut facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        facts.refund_not_on_a_1099g = dec!(250);
        // The caller transcribed $900 of Form 1099-G box 2; the filer also got $250 with no form.
        let d = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("{d:?}")
        };
        assert_eq!(
            sheet.line1,
            dec!(1150),
            "both halves of line 1's first sentence"
        );
        assert_eq!(part, TaxablePart::Amount(dec!(1150)));
        // ★ And it is still subject to line 1's OWN cap, which is the whole point of adding before
        //   capping rather than after: capping each half separately would let a filer past the cap.
        facts.prior_year_schedule_a_line5d = dec!(1000);
        facts.prior_year_schedule_a_line5e = dec!(1000);
        let d2 = figure(2025, Some(true), dec!(900), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet: s2, .. } = d2 else {
            panic!("{d2:?}")
        };
        assert_eq!(s2.line1, dec!(1000), "capped at line 5d, not 900 + 250");
    }

    /// ★★ **Line 6's boxes move the answer**, and the aged/blind pair is the branch a closed form
    /// drops. Same filer as vector 2 but 65 and blind: line 6 adds 2 × $1,550, line 7 becomes
    /// $32,300, which is MORE than the $30,000 itemized total — so the benefit vanishes and the STOP
    /// at line 8 is reached.
    #[test]
    fn line6s_boxes_can_erase_the_benefit_entirely() {
        let mut facts = itemizer(FilingStatus::Mfj, dec!(9000), dec!(9000), dec!(30000));
        facts.prior_year_aged_blind.taxpayer_aged = true;
        facts.prior_year_aged_blind.taxpayer_blind = true;
        let d = figure(2025, Some(true), dec!(3000), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("{d:?}")
        };
        assert_eq!(sheet.line6, Some(dec!(3100)), "2 boxes × $1,550");
        assert_eq!(sheet.line7, Some(dec!(32300)));
        assert_eq!(part, TaxablePart::Nothing(StopLine::Line8));
        assert_eq!(d.schedule_1_line1(), None);
    }

    /// ★★★ **The MFS Note skips lines 5 through 7** — *"enter the amount from line 4 on line 8, and
    /// go to line 9"*. The skipped lines are `None`, not `-0-`, and line 8 is line 4 with no
    /// comparison made.
    #[test]
    fn the_mfs_note_skips_lines_5_through_7() {
        let mut facts = itemizer(FilingStatus::Mfs, dec!(6000), dec!(5000), dec!(9000));
        facts.prior_year_mfs_spouse_itemized = true;
        let d = figure(2025, Some(true), dec!(1500), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { sheet, part } = d else {
            panic!("{d:?}")
        };
        assert_eq!(sheet.line2, Some(dec!(1000)), "6,000 − 5,000");
        assert_eq!(sheet.line3, Some(dec!(500)), "1,500 − 1,000");
        assert_eq!(sheet.line4, Some(dec!(9000)));
        assert_eq!(sheet.line5, None, "skipped by the Note — not -0-");
        assert_eq!(sheet.line6, None);
        assert_eq!(sheet.line7, None);
        assert_eq!(sheet.line8, Some(dec!(9000)), "line 4 entered on line 8");
        assert_eq!(sheet.line9, Some(dec!(500)), "the smaller of 500 and 9,000");
        assert_eq!(part, TaxablePart::Amount(dec!(500)));
        // ★ And WITHOUT the Note's condition the same filer works lines 5-7: MFS takes the $14,600
        //   bullet, which exceeds the $9,000 itemized total, so the STOP at line 8 is reached and
        //   NOTHING is taxable. The Note is not cosmetic — it is the difference between $500 of income
        //   and none.
        facts.prior_year_mfs_spouse_itemized = false;
        let d2 = figure(2025, Some(true), dec!(1500), Some(&facts)).expect("the worksheet applies");
        let Decision::Worksheet { part: p2, .. } = d2 else {
            panic!("{d2:?}")
        };
        assert_eq!(p2, TaxablePart::Nothing(StopLine::Line8));
    }

    /// ★★★ **A prior-year MFS filer's SPOUSE boxes fail closed** unless the footnote's three
    /// conditions are affirmed. Over-claiming them raises line 7 and SHRINKS the taxable part, so the
    /// direction of the default is the whole point.
    #[test]
    fn an_mfs_filers_spouse_boxes_are_forgone_unless_the_footnote_is_affirmed() {
        let boxes = PriorYearAgedBlindBoxes {
            taxpayer_aged: true,
            taxpayer_blind: false,
            spouse_aged: true,
            spouse_blind: true,
        };
        assert_eq!(
            boxes.count(FilingStatus::Mfs, false),
            1,
            "the two spouse boxes are FORGONE — the footnote was not affirmed"
        );
        assert_eq!(
            boxes.count(FilingStatus::Mfs, true),
            3,
            "…and counted when it is"
        );
        assert_eq!(
            boxes.count(FilingStatus::Mfj, false),
            3,
            "the footnote is an MFS rule only; a joint filer's spouse boxes always count"
        );
    }

    /// ★★★ **Every one of the nine Exception conditions refuses, one at a time.** A condition that was
    /// modelled and never wired would be an instrument that cannot fire; this walks
    /// [`Pub525Exception::ALL`] and demands a refusal for each, so no variant can be inert.
    #[test]
    fn each_of_the_nine_exceptions_refuses_the_worksheet() {
        for e in Pub525Exception::ALL {
            let mut facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
            match e {
                Pub525Exception::E1RefundForAnotherYear => {
                    facts.exception_refund_for_another_year = true;
                }
                Pub525Exception::E2NotAnIncomeTaxRefund => {
                    facts.exception_not_an_income_tax_refund = true;
                }
                Pub525Exception::E3ZeroRateOnPreferentialIncome => {
                    facts.exception_zero_rate_on_preferential_income = true;
                }
                Pub525Exception::E4RefundExceedsIncrementalDeduction => {
                    facts.exception_refund_exceeds_incremental_deduction = true;
                }
                Pub525Exception::E5LastEstimatedPaymentInFilingYear => {
                    facts.exception_last_estimated_payment_in_filing_year = true;
                }
                Pub525Exception::E6OwedAmtInPriorYear => {
                    facts.exception_owed_amt_in_prior_year = true;
                }
                Pub525Exception::E7UnusableCredits => {
                    facts.exception_unusable_credits = true;
                }
                Pub525Exception::E8CouldBeClaimedAsDependent => {
                    facts.exception_could_be_claimed_as_dependent = true;
                }
                Pub525Exception::E9JointStateReturnNotJointNow => {
                    facts.exception_joint_state_return_not_joint_now = true;
                }
            }
            assert_eq!(
                facts.exception_that_applies(),
                Some(e),
                "{e:?} must be the condition the walk reports"
            );
            assert_eq!(
                figure(2025, Some(true), dec!(900), Some(&facts)),
                Err(NotUsable::Pub525Exception(e)),
                "{e:?} must REFUSE — Pub. 525's Itemized Deduction Recoveries governs, and btctax \
                 models no part of it"
            );
            for rev in REVISIONS {
                assert!(
                    rev.exception_text(e).is_some(),
                    "TY{} prints no text for {e:?}",
                    rev.year
                );
            }
        }
    }

    /// ★★★ **A year with no archived revision REFUSES rather than borrowing a neighbour's
    /// constants.** TY2026's `i1040gi` does not exist until the January-2027 finals, and lines 5 and 6
    /// carry that year's figures — nothing else can supply them.
    #[test]
    fn a_year_with_no_archived_revision_refuses() {
        let facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        let unarchived = REVISIONS.iter().map(|r| r.year).max().expect("non-empty") + 1;
        assert_eq!(
            figure(unarchived, Some(true), dec!(900), Some(&facts)),
            Err(NotUsable::NoArchivedRevision {
                tax_year: unarchived
            })
        );
    }

    /// ★★★ **An uncollected block REFUSES**, which is the state every existing vault is in — so the
    /// worksheet cannot silently answer for a filer who was never asked.
    #[test]
    fn uncollected_facts_refuse() {
        assert_eq!(
            figure(2025, Some(true), dec!(900), None),
            Err(NotUsable::FactsNotCollected)
        );
        // ★ An UNANSWERED prior-year-itemize declaration does not sneak through as a TIP exit either.
        assert_eq!(
            figure(2025, None, dec!(900), None),
            Err(NotUsable::FactsNotCollected)
        );
    }

    /// ★★ **The whole block round-trips through serde, and a TOML that OMITS a field refuses to
    /// parse.** That is what makes the inner `bool`s class-`SerdeRequired` rather than a launderable
    /// default: there is no `false` for an unanswered box to hide in.
    #[test]
    fn an_omitted_field_refuses_to_deserialize() {
        let facts = itemizer(FilingStatus::Single, dec!(8000), dec!(8000), dec!(20000));
        let json = serde_json::to_string(&facts).expect("serializes");
        let back: StateLocalRefundFacts = serde_json::from_str(&json).expect("round-trips");
        assert_eq!(back, facts);
        let gutted = json.replace("\"exception_owed_amt_in_prior_year\":false,", "");
        assert_ne!(gutted, json, "the field must actually have been removed");
        let err = serde_json::from_str::<StateLocalRefundFacts>(&gutted)
            .expect_err("a block missing an exception answer must NOT parse");
        assert!(
            err.to_string().contains("exception_owed_amt_in_prior_year"),
            "the error must name the missing field: {err}"
        );
    }
}
