//! **Pub. 936 TABLE 1 — *Worksheet To Figure Your Qualified Loan Limit and Deductible Home
//! Mortgage Interest for the Current Year***, transcribed.
//!
//! ★★★ **WHY THIS EXISTS (FR-200a).** A filer whose mortgage is over a §163(h)(3)(B) ceiling could
//! not file **anything** — `RefuseReason::MortgageOverDebtLimit` refused the whole packet, Form 8949
//! and Schedule D included, which is the part this product exists to produce. The refusal's own doc
//! said why: *"i1040sca's Limits on home mortgage interest block states four limits and btctax
//! models only the mixed-use one."* So the product asked the filer whether a limit bit, and then
//! refused when they said yes, because it could not do the arithmetic. This module is the
//! arithmetic. It was never a hard tax question — it is sixteen lines of *"enter the smaller of"*
//! and *"divide line 11 by line 12"* that nobody had typed in.
//!
//! **i1040sca does NOT contain it.** The *Limits on home mortgage interest* block states the four
//! limits and says *"see Pub. 936 to figure your deduction"* four times
//! (`design/forms/extract/i1040sca--2024.txt:877` onward). Pub. 936 is the authority, and its Table 1
//! is the instrument.
//!
//! **Transcribed, not derived** (`CLAUDE.md`): one field per numbered line, named for the line, in
//! the form's own numbering, carrying the official instruction text verbatim as its doc comment. No
//! closed form. **The BRANCHES are lines of the form too** — Part I's two-bullet jump after line 6
//! and Part II's *"stop here"* after line 12 — and a line the form SKIPS is `None`, never `-0-`,
//! because *"two blanks look identical on the printed page and are not the same thing"*.
//!
//! ★★★ **THE ORACLES CANNOT CHECK ANY OF THIS**, and that has to be said out loud rather than left
//! to a reader to infer. Both OpenTaxSolver and Tax-Calculator consume Schedule A **line 8a as an
//! INPUT** (`FOLLOWUPS.md` §G-9: *a value the oracles take as input is never validated by their
//! agreement*), so a green two-oracle sweep proves nothing about a single line below. The validation
//! is this module's KATs, whose expected figures come from **the publication's own worked example**
//! and from **i1040sca's own sentence** about how the two ceilings interact — never from this
//! implementation.
//!
//! ## What now computes, and what still refuses
//!
//! i1040sca names **four** limits. After this module:
//!
//! | limit | status |
//! |---|---|
//! | loan proceeds not used to buy/build/improve (the mixed-use limit) | already modelled — §163(h)(3)(F), the line-8 checkbox and `mortgage_all_used_to_buy_build_improve` |
//! | debt taken out on or before Dec 15, 2017 — $1,000,000 / $500,000 MFS | **COMPUTES** (lines 2, 3, 4, 5, 6) |
//! | debt taken out after Dec 15, 2017 — $750,000 / $375,000 MFS, reduced by the older debt | **COMPUTES** (lines 7, 8, 9, 10, 11) |
//! | mortgages exceed the home's fair market value | **REFUSES** — [`NotUsable::FairMarketValueLimit`] |
//!
//! ★★ **The fair-market-value limit is not an omission here, it is a hole in the publication.**
//! Pub. 936 (2025) prints **no worksheet for it and does not mention it** — the only occurrences of
//! *"fair market value"* in the whole document are the cost-allocation rule for a partly-non-qualified
//! home and one figure inside an example. i1040sca still says *"Limit when loans exceed the fair
//! market value of the home. If the total amount of all mortgages is more than the fair market value
//! of the home, see Pub. 936 to figure your deduction."* (`i1040sca--2025.txt:1054`), and Pub. 936
//! does not figure it. That is a pre-TCJA instruction whose worksheet went away with home-equity-debt
//! deductibility. btctax will not invent one, so an affirmed FMV limit refuses and says so.
//!
//! ★★ **The April 2018 transition rule also refuses** ([`NotUsable::April2018TransitionRule`]).
//! Figure A's footnote 3: *"A taxpayer who enters into a written binding contract before December
//! 15, 2017, to close on the purchase of a principal residence before January 1, 2018, and who
//! purchases such residence before April 1, 2018, is considered to have incurred the home
//! acquisition debt prior to December 16, 2017, and may use the 2017 threshold amounts of $1,000,000
//! ($500,000 for married filing separately)."* It RE-BUCKETS a balance from line 7 to line 2, which
//! is the **filer-favourable** direction — so ignoring it would overstate the tax, and silently
//! applying it would understate it. btctax models neither, and refuses rather than pick.
//! (`widening-an-exemption-is-never-the-safe-edit`.)
//!
//! ## The two "you don't need Table 1" situations are the line-12 STOP
//!
//! The Table 1 Instructions open with an exit: *"You can deduct all of the interest you paid during
//! the year on mortgages secured by your main home or second home in either of the following two
//! situations."* — *"All the mortgages are grandfathered debt."* and *"The total of the mortgage
//! balances for the entire year is within the limits discussed earlier under Home Acquisition
//! Debt."* — *"In either of those cases, you don't need Table 1."*
//!
//! ★★★ **Both are exactly the state Part II's own STOP produces**, which is why this module has no
//! separate exit for them and why running the worksheet unconditionally is never wrong. Proved, not
//! asserted, and pinned by `tests::the_all_grandfathered_situation_reaches_the_line_12_stop` and
//! `tests::the_within_the_limits_situation_reaches_the_line_12_stop`:
//!
//! - *all grandfathered* ⇒ lines 2 and 7 are `0` and line 12 is line 1, so line 5 = line 1, line 4 =
//!   max(line 1, $1,000,000) ≥ line 1, line 6 = min(line 4, line 5) = line 1 = line 11 = line 12 ⇒
//!   **line 11 ≥ line 12 ⇒ STOP**;
//! - *within the limits* ⇒ line 11 is the limit and line 12 is the balance, and the situation says
//!   the balance is inside the limit ⇒ **line 11 ≥ line 12 ⇒ STOP**.
//!
//! ## Line 14's rounding, which is a money decision and therefore stated
//!
//! *"Divide the amount on line 11 by the amount on line 12. Enter the result as a decimal amount
//! (rounded to three places)."* The form says *"rounded"* and names no tie-break. This module uses
//! the IRS's own stated convention — half-up, away from zero,
//! [`DOLLAR_ROUNDING`](crate::conventions::DOLLAR_ROUNDING) — because that is what i1040 prescribes
//! for rounding on a return (*"drop amounts under 50 cents and increase amounts from 50 to 99 cents
//! to the next dollar"*). A tie at the fourth decimal place is the only place the choice is
//! observable, and it moves line 15 by at most `0.0005 × line 13` in the filer's favour.
//!
//! ★ **Line 14 can never divide by zero**, and that is a property of the form rather than a guard
//! bolted on: line 11 ≥ 0 always (line 9 ≥ $375,000 and line 10 ≥ 0), so line 12 = 0 ⇒ line 11 ≥
//! line 12 ⇒ the STOP fires and lines 13–16 are never reached.
//! `tests::a_zero_line_12_stops_before_the_division` pins it, and the division still goes through
//! `checked_div` because this crate's compute core does not panic.

use crate::conventions::{Usd, DOLLAR_ROUNDING};
use crate::tax::types::FilingStatus;
use serde::{Deserialize, Serialize};

/// The archived text layer this transcription was typed from, relative to the repo root.
///
/// ★ **Transcribed from the TEXT LAYER, never from the rendered page** (`CLAUDE.md`). A rendered
/// `12` and `22` differ by a few pixels, and reading Form 6251 line 33 off an image once produced
/// *"Subtract line 32 from line 12"* where the form says **line 22** — a $200,000 error on one
/// vector.
///
/// ★ The quotes travel as `&'static str` rather than through `include_str!` for
/// `state_local_refund`'s reason: `legal/` is outside this published crate, and a reach out of the
/// crate root ships a tarball that builds in the workspace and is broken for everyone else, **with
/// exit 0**. The checking lives in this module's `#[cfg(test)]` block, which may read the repo tree.
pub const EXTRACT_PATH: &str =
    "legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt";

/// The directory every archived revision of this publication lives in, relative to the repo root.
///
/// ★★★ **There is no per-year revision table here, and that is a stated boundary rather than an
/// omission** (`CLAUDE.md`, *"derive the list, or make the compiler hold it — never type one beside
/// a set that grows"*, option 3: *state, in the source, exactly what it covers and what it does
/// not*). Every constant Table 1 prints is **statutory and unindexed** — §163(h)(3)(B)(ii)'s
/// $1,000,000/$500,000, §163(h)(3)(F)(i)(II)'s $750,000/$375,000, and the two bucket dates — and
/// P.L. 119-21 made the second pair permanent (the publication's own *Tax reform* note). Contrast
/// `state_local_refund`, whose lines 5 and 6 print a *year's* standard deduction and therefore needs
/// one entry per revision.
///
/// ★ What holds the boundary is `tests::every_archived_revision_prints_every_quote`: it reads this
/// directory, and a future `Pub936_…` extract that reworded a single line reds this module until
/// someone re-transcribes it. The set is derived; nothing is typed beside it.
pub const EXTRACT_DIR: &str = "legal/text/irs-publications";

/// The filename prefix every archived revision of Pub. 936 shares.
pub const EXTRACT_PREFIX: &str = "Pub936_Home_Mortgage_Interest_Deduction";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The un-numbered text: the branches, the STOP, and the Instructions' own exit.
//
// ★★★ **THESE ARE DATA BECAUSE A TEST MUST WALK THEM.** The sixteen numbered lines carry their text
//     as doc comments on [`Table1`], and `tests::every_numbered_line_doc_comment_is_verbatim_in_the_extract`
//     parses THIS FILE'S OWN SOURCE to check them — so there is exactly one copy of each and it
//     cannot drift. The branches cannot be done that way: they belong to no field. They live here as
//     `&'static str`, and TWO tests hold them:
//
//     - `every_branch_and_stop_is_verbatim_in_the_extract` — the text is the form's, whole;
//     - `every_branch_text_appears_verbatim_in_figures_own_comments` — `figure`'s inline narration of
//       each branch is byte-identical to the checked string, so editing the comment reds.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Part I's first branch bullet, after line 6 — the jump that skips lines 7 through 10.
pub const BRANCH_NO_POST_2017_DEBT: &str = "If you have no home acquisition debt incurred after \
    December 15, 2017, or the amount on line 6 is $750,000 ($375,000 if married filing separately) \
    or more, line 6 is your qualified loan limit. Enter this amount on line 11 and go to Part II, \
    line 12.";

/// Part I's second branch bullet, after line 6 — the limb that works lines 7 through 11.
pub const BRANCH_HAS_POST_2017_DEBT: &str =
    "If you have home acquisition debt incurred after December 15, 2017, go to line 7.";

/// Part II's first bullet, after line 12 — the limb that works lines 13 through 16.
pub const BRANCH_LIMIT_BINDS: &str = "If line 11 is less than line 12, go on to line 13.";

/// Part II's STOP, after line 12.
pub const BRANCH_STOP_ALL_DEDUCTIBLE: &str = "If line 11 is equal to or more than line 12, stop \
    here. All of your interest on all the mortgages included on line 12 is deductible as home \
    mortgage interest on Schedule A (Form 1040).";

/// Every un-numbered branch of the worksheet, in the form's own order.
///
/// ★ A `const` array rather than a hand-list at each call site, so
/// `tests::every_branch_and_stop_is_verbatim_in_the_extract` walks the set rather than a copy of it.
pub const BRANCHES: &[&str] = &[
    BRANCH_NO_POST_2017_DEBT,
    BRANCH_HAS_POST_2017_DEBT,
    BRANCH_LIMIT_BINDS,
    BRANCH_STOP_ALL_DEDUCTIBLE,
];

/// The Table 1 Instructions' own opening exit, and its two situations.
///
/// ★★★ Both situations are exactly the state [`BRANCH_STOP_ALL_DEDUCTIBLE`] produces, which is why
/// this module models no separate exit for them. See the module docs for the proof.
pub const NO_TABLE_1_NEEDED: &[&str] = &[
    "You can deduct all of the interest you paid during the year on mortgages secured by your main \
     home or second home in either of the following two situations.",
    "All the mortgages are grandfathered debt.",
    "The total of the mortgage balances for the entire year is within the limits discussed earlier \
     under Home Acquisition Debt.",
    "In either of those cases, you don't need Table 1.",
];

/// The Table 1 Instructions' per-line paragraphs, `(line, text)`, verbatim.
///
/// ★ Only the lines the publication prints an instruction FOR. The rest are self-contained on the
/// worksheet (*"Enter the smaller of the amount on line 4 or the amount on line 5"* needs none), and
/// `tests::the_instructed_line_set_is_read_off_the_form` derives which those are from the extract
/// rather than trusting this list — the T8/FR-99 repair.
pub const LINE_INSTRUCTIONS: &[(u8, &str)] = &[
    (
        1,
        "Figure the average balance for the current year of each mortgage you had on all qualified \
         homes on October 13, 1987 (grandfathered debt). Add the results together and enter the \
         total on line 1. Include the average balance for the current year for any grandfathered \
         debt that is part of a mixed-use mortgage.",
    ),
    (
        2,
        "Figure the average balance for the current year of each mortgage you took out on all \
         qualified homes after October 13, 1987, and prior to December 16, 2017, to buy, build, or \
         substantially improve the home (home acquisition debt). Add the results together and enter \
         the total on line 2. Include the average balance for the current year for any home \
         acquisition debt that is part of a mixed-use mortgage.",
    ),
    (
        7,
        "Figure the average balance for the current year of each mortgage you took out on all \
         qualified homes after December 15, 2017, to buy, build, or substantially improve the home \
         (home acquisition debt). Add the results together and enter the total on line 7.",
    ),
    (
        12,
        "Figure the average balance for the current year of each outstanding home mortgage. Add the \
         average balances together and enter the total on line 12. See Average Mortgage Balance, \
         earlier.",
    ),
    (
        13,
        "If you make payments to a financial institution, or to a person whose business is making \
         loans, you should get Form 1098 or a similar statement from the lender. This form will show \
         the amount of interest to enter on line 13. Also, include on this line any other interest \
         payments made on debts secured by a qualified home for which you didn't receive a Form \
         1098. Don't include points or mortgage insurance premiums on this line.",
    ),
    (
        16,
        "You can't deduct the amount of interest on line 16 as home mortgage interest. If you didn't \
         use any of the proceeds of any mortgage included on line 12 of the worksheet for business, \
         investment, or other deductible activities, then all the interest on line 16 is personal \
         interest. Personal interest isn't deductible.",
    ),
];

/// Line 12's Note — the sentence that puts **home equity debt** in line 12's average and therefore
/// makes line 12 a SUPERSET of lines 1, 2 and 7. See [`Table1Facts::line12_all_mortgages`].
pub const LINE_12_NOTE: &str =
    "Note: If the average balance consists of more than one category of \
    debt (grandfathered debt, home acquisition debt, and home equity debt), see Mixed-use \
    mortgages, earlier, to figure the average mortgage balance.";

/// *Claiming your deductible points*, item 2 — the publication's own rule for applying line 14 to a
/// component that never passes through line 13. It is what [`apportion`] implements.
pub const POINTS_TIMES_LINE_14: &str = "Multiply the amount in item 1 by the decimal amount on line \
    14. Enter the result on Schedule A (Form 1040), line 8a or 8c, whichever applies. This amount is \
    fully deductible.";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The form's own printed constants, and its two halved-for-MFS pairs.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **Line 3** — *"Enter $1,000,000 ($500,000 if married filing separately)"*. §163(h)(3)(B)(ii).
#[must_use]
pub fn line3_amount(status: FilingStatus) -> Usd {
    if status == FilingStatus::Mfs {
        Usd::from(500_000)
    } else {
        Usd::from(1_000_000)
    }
}

/// **Line 8** — *"Enter $750,000 ($375,000 if married filing separately)"*. §163(h)(3)(F)(i)(II).
#[must_use]
pub fn line8_amount(status: FilingStatus) -> Usd {
    if status == FilingStatus::Mfs {
        Usd::from(375_000)
    } else {
        Usd::from(750_000)
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The collected facts — what the worksheet asks for that nothing on the return could answer.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **What Table 1 asks the filer for.** Every field is a line of the worksheet or a condition the
/// instructions make the filer test, and none of it is derivable from anything btctax already holds:
/// it collects mortgage INTEREST (Form 1098 box 1), never an average balance.
///
/// ★★★ *"If the form asks something our input surface cannot answer, collect it. That is following
/// instructions, not scope creep."* (`CLAUDE.md`.)
///
/// ★★★ **A MISSING FIGURE IS NOT ZERO.** The whole block is `Option` on the Schedule A
/// ([`ScheduleAInputs::pub936_table1`](crate::tax::return_inputs::ScheduleAInputs::pub936_table1))
/// and its absence is [`NotUsable::FactsNotCollected`], which REFUSES — *"an entry is testimony"*,
/// and a `0` on an unasked average balance would fabricate the filer's testimony in the
/// understatement direction (a zero line 12 makes every dollar of interest deductible). There is no
/// `#[serde(default)]` on any money leaf below for the same reason: a block present but missing a
/// balance is a mistyped block, not a lawful state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table1Facts {
    /// **Line 1** — *"Enter the average balance of all your grandfathered debt. See the line 1
    /// instructions"*.
    ///
    /// The line 1 instructions: *"Figure the average balance for the current year of each mortgage
    /// you had on all qualified homes on October 13, 1987 (grandfathered debt). Add the results
    /// together and enter the total on line 1. Include the average balance for the current year for
    /// any grandfathered debt that is part of a mixed-use mortgage."*
    pub line1_grandfathered: Usd,
    /// **Line 2** — *"Enter the average balance of all your home acquisition debt incurred after
    /// October 13, 1987, and prior to December 16, 2017. See the line 2 instructions"*.
    ///
    /// The line 2 instructions: *"Figure the average balance for the current year of each mortgage
    /// you took out on all qualified homes after October 13, 1987, and prior to December 16, 2017,
    /// to buy, build, or substantially improve the home (home acquisition debt). Add the results
    /// together and enter the total on line 2. Include the average balance for the current year for
    /// any home acquisition debt that is part of a mixed-use mortgage."*
    pub line2_acquisition_before_dec_16_2017: Usd,
    /// **Line 7** — *"Enter the average balance of all your home acquisition debt incurred after
    /// December 15, 2017. See the line 7 instructions"*.
    ///
    /// The line 7 instructions: *"Figure the average balance for the current year of each mortgage
    /// you took out on all qualified homes after December 15, 2017, to buy, build, or substantially
    /// improve the home (home acquisition debt). Add the results together and enter the total on
    /// line 7."*
    pub line7_acquisition_after_dec_15_2017: Usd,
    /// **Line 12** — *"Enter the total of the average balances of all mortgages from lines 1, 2, and
    /// 7 on all qualified homes. See the line 12 instructions"*.
    ///
    /// The line 12 instructions read WIDER than the line itself: *"Figure the average balance for
    /// the current year of each outstanding home mortgage. Add the average balances together and
    /// enter the total on line 12. See Average Mortgage Balance, earlier."* plus *"Note: If the
    /// average balance consists of more than one category of debt (grandfathered debt, home
    /// acquisition debt, and home equity debt), see Mixed-use mortgages, earlier, to figure the
    /// average mortgage balance."*
    ///
    /// ★★★ **COLLECTED, NOT DERIVED AS `line1 + line2 + line7`, and the two texts are why.** The
    /// worksheet line names lines 1, 2 and 7; the instruction names *every outstanding home
    /// mortgage*, and the Note puts **home equity debt** — which is in none of lines 1, 2 and 7 — in
    /// the same average. So the instruction's set is a SUPERSET, and a derived `1 + 2 + 7` would
    /// silently drop home equity debt out of line 14's denominator, making the ratio too LARGE and
    /// the deduction too big. Understatement, invisible, and exactly the compression `CLAUDE.md`
    /// forbids.
    ///
    /// ★★ What the two readings jointly prove is a BOUND, and that bound is enforced:
    /// `line12 >= line1 + line2 + line7` must hold, or [`NotUsable::Line12BelowItsComponents`]
    /// refuses. A figure below the sum contradicts the worksheet's own sentence; a figure above it is
    /// the home-equity case the Note contemplates, and it can only shrink the deduction.
    pub line12_all_mortgages: Usd,
    /// **The fair-market-value limit** — i1040sca: *"Limit when loans exceed the fair market value of
    /// the home. If the total amount of all mortgages is more than the fair market value of the home,
    /// see Pub. 936 to figure your deduction."* (`i1040sca--2025.txt:1054`.)
    ///
    /// ★★★ `true` ⇒ [`NotUsable::FairMarketValueLimit`] REFUSES. Pub. 936 (2025) contains no
    /// worksheet for this limit and does not mention it; see the module docs. Phrased so `false` —
    /// *"my mortgages did not exceed the home's value"* — is the ordinary answer, so a filer who
    /// simply has a large mortgage is not walled by a limit that almost never applies after TCJA.
    pub mortgages_exceed_fair_market_value: bool,
    /// **Figure A, footnote 3** / i1040sca's *"An exception exists for certain loans taken out after
    /// December 15, 2017, but before April 1, 2018"*: *"A taxpayer who enters into a written binding
    /// contract before December 15, 2017, to close on the purchase of a principal residence before
    /// January 1, 2018, and who purchases such residence before April 1, 2018, is considered to have
    /// incurred the home acquisition debt prior to December 16, 2017, and may use the 2017 threshold
    /// amounts of $1,000,000 ($500,000 for married filing separately)."*
    ///
    /// ★★★ `true` ⇒ [`NotUsable::April2018TransitionRule`] REFUSES, and the direction is the reason.
    /// The rule moves a balance from line 7 to line 2 — from the $750,000 bucket to the $1,000,000
    /// one — which is FILER-FAVOURABLE. Ignoring it overstates the tax; applying it needs to know
    /// which loan and how much, which btctax does not collect. So it refuses rather than pick a
    /// direction for the filer.
    pub april_2018_binding_contract: bool,
    /// Whether these figures were typed by the filer or carried from a return btctax itself
    /// computed. ★ Present from the first commit for §G-23's *"stated zero"* reason: a `0` on line 1
    /// is a real and overwhelmingly common figure, and *"the filer said zero"* must not be the same
    /// bytes as *"btctax computed zero"*.
    #[serde(default)]
    pub provenance: crate::tax::return_inputs::CarryProvenance,
}

impl Table1Facts {
    /// The bound both readings of line 12 jointly prove: line 12 is at least lines 1 + 2 + 7.
    #[must_use]
    pub fn line12_components(&self) -> Usd {
        self.line1_grandfathered
            + self.line2_acquisition_before_dec_16_2017
            + self.line7_acquisition_after_dec_15_2017
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The worksheet.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **Pub. 936 Table 1**, one field per numbered line, in the form's own numbering.
///
/// A line the form SKIPS is `None`, never `-0-`. Two skips exist:
///
/// - **lines 7–10**, on Part I's first branch bullet (*"…line 6 is your qualified loan limit. Enter
///   this amount on line 11 and go to Part II, line 12."*);
/// - **lines 13–16**, on Part II's STOP (*"stop here. All of your interest … is deductible"*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table1 {
    /// L1 — "Enter the average balance of all your grandfathered debt. See the line 1
    ///       instructions"
    pub line1: Usd,
    /// L2 — "Enter the average balance of all your home acquisition debt incurred after October
    ///       13, 1987, and prior to December 16, 2017. See the line 2 instructions"
    pub line2: Usd,
    /// L3 — "Enter $1,000,000 ($500,000 if married filing separately)"
    pub line3: Usd,
    /// L4 — "Enter the larger of the amount on line 1 or the amount on line 3"
    pub line4: Usd,
    /// L5 — "Add the amounts on lines 1 and 2. Enter the total here"
    pub line5: Usd,
    /// L6 — "Enter the smaller of the amount on line 4 or the amount on line 5"
    pub line6: Usd,
    /// L7 — "Enter the average balance of all your home acquisition debt incurred after December
    ///       15, 2017. See the line 7 instructions"
    ///
    /// `None` = the first branch bullet after line 6 sent the filer straight to line 11.
    pub line7: Option<Usd>,
    /// L8 — "Enter $750,000 ($375,000 if married filing separately)"
    ///
    /// `None` = skipped by the same branch.
    pub line8: Option<Usd>,
    /// L9 — "Enter the larger of the amount on line 6 or the amount on line 8"
    ///
    /// `None` = skipped by the same branch.
    pub line9: Option<Usd>,
    /// L10 — "Add the amounts on lines 6 and 7. Enter the total here"
    ///
    /// `None` = skipped by the same branch.
    pub line10: Option<Usd>,
    /// L11 — "Enter the smaller of line 9 or line 10. This is your qualified loan limit"
    ///
    /// ★ Never `None`: the branch's own instruction is *"Enter this amount on line 11"*, so line 11
    /// is filled on both limbs. That is the whole point of the jump.
    pub line11: Usd,
    /// L12 — "Enter the total of the average balances of all mortgages from lines 1, 2, and 7 on all
    ///        qualified homes. See the line 12 instructions"
    pub line12: Usd,
    /// L13 — "Enter the total amount of interest that you paid on the loans from line 12. See the
    ///        line 13 instructions"
    ///
    /// `None` = the STOP after line 12 was reached and the whole of Part II's arithmetic is skipped.
    pub line13: Option<Usd>,
    /// L14 — "Divide the amount on line 11 by the amount on line 12. Enter the result as a decimal
    ///        amount (rounded to three places)"
    ///
    /// ★ NOT a money amount — a bare decimal, three places. See the module docs on its rounding.
    pub line14: Option<Usd>,
    /// L15 — "Multiply the amount on line 13 by the decimal amount on line 14. Enter the result.
    ///        This is your deductible home mortgage interest. Enter this amount on
    ///        Schedule A (Form 1040)"
    pub line15: Option<Usd>,
    /// L16 — "Subtract the amount on line 15 from the amount on line 13. Enter the result. This
    ///        isn't home mortgage interest. See the line 16 instructions"
    ///
    /// The line 16 instructions: *"You can't deduct the amount of interest on line 16 as home
    /// mortgage interest. If you didn't use any of the proceeds of any mortgage included on line 12
    /// of the worksheet for business, investment, or other deductible activities, then all the
    /// interest on line 16 is personal interest. Personal interest isn't deductible."*
    pub line16: Option<Usd>,
}

/// Which of Part II's two outcomes the worksheet reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The STOP after line 12: *"If line 11 is equal to or more than line 12, stop here. All of your
    /// interest on all the mortgages included on line 12 is deductible as home mortgage interest on
    /// Schedule A (Form 1040)."*
    ///
    /// ★ This is also both of the Table 1 Instructions' *"you don't need Table 1"* situations; see
    /// the module docs.
    AllDeductible,
    /// Lines 13–16 were worked: line 11 was less than line 12, so only line 14's fraction of the
    /// interest is home mortgage interest.
    Limited {
        /// Line 14 — the decimal, rounded to three places, that every component is multiplied by.
        ratio: Usd,
        /// Line 15 — *"This is your deductible home mortgage interest."*
        deductible: Usd,
        /// Line 16 — *"This isn't home mortgage interest."*
        not_home_mortgage_interest: Usd,
    },
}

/// A worked Table 1: every line as filled or skipped, plus which outcome it reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Worked {
    /// Every numbered line.
    pub sheet: Table1,
    /// Part II's outcome.
    pub outcome: Outcome,
}

impl Worked {
    /// The fraction of every interest and points component that is deductible home mortgage
    /// interest: line 14 when Part II was worked, and `1` on the STOP (*"All of your interest … is
    /// deductible"*).
    #[must_use]
    pub fn ratio(&self) -> Usd {
        match self.outcome {
            Outcome::AllDeductible => Usd::ONE,
            Outcome::Limited { ratio, .. } => ratio,
        }
    }
}

/// Why Table 1 cannot be worked, so btctax must refuse rather than print a figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotUsable {
    /// The filer has not supplied Table 1's figures —
    /// [`ScheduleAInputs::pub936_table1`](crate::tax::return_inputs::ScheduleAInputs::pub936_table1)
    /// is `None`. UNANSWERED, in this repo's own sense: nothing ever populated it.
    FactsNotCollected,
    /// i1040sca's FOURTH limit, for which Pub. 936 (2025) prints no worksheet. See the module docs.
    FairMarketValueLimit,
    /// Figure A's footnote 3 — the written-binding-contract exception that re-buckets a balance from
    /// line 7 to line 2. btctax models neither direction.
    April2018TransitionRule,
    /// Line 12 is below lines 1 + 2 + 7, which contradicts the worksheet's own sentence for that
    /// line. Refused rather than silently clamped, because a clamp would print a deduction from a
    /// figure the form says cannot exist.
    Line12BelowItsComponents {
        /// The figure supplied for line 12.
        line12: Usd,
        /// Lines 1 + 2 + 7, which line 12's own text says it is the total of.
        components: Usd,
    },
    /// Line 14's division did not evaluate. Unreachable — line 12 = 0 forces the STOP before line 14
    /// exists (see the module docs) — and carried as a value rather than a panic because this
    /// crate's compute core does not panic.
    Line14DivisionFailed {
        /// Line 11, the dividend.
        line11: Usd,
        /// Line 12, the divisor.
        line12: Usd,
    },
}

/// **Work Table 1** — or say why it cannot be worked.
///
/// `status` selects the halved-for-MFS figures the form prints on lines 3 and 8. `line13` is the
/// interest paid on the loans on line 12, which the caller derives from the return's own documents
/// ([`line13_interest_paid`]) rather than collecting a second time.
pub fn figure(
    status: FilingStatus,
    facts: Option<&Table1Facts>,
    line13: Usd,
) -> Result<Worked, NotUsable> {
    let f = facts.ok_or(NotUsable::FactsNotCollected)?;
    // ★★ The two limits Pub. 936 does NOT figure, BEFORE any arithmetic. Each is a refusal rather
    //    than a term, and each would move the answer if it applied — so computing past either would
    //    print a figure that is wrong in a direction btctax could not name.
    if f.mortgages_exceed_fair_market_value {
        return Err(NotUsable::FairMarketValueLimit);
    }
    if f.april_2018_binding_contract {
        return Err(NotUsable::April2018TransitionRule);
    }
    // ★ Line 12's own sentence makes it the total of lines 1, 2 and 7, so a smaller figure is a
    //   contradiction. See `Table1Facts::line12_all_mortgages`.
    let components = f.line12_components();
    if f.line12_all_mortgages < components {
        return Err(NotUsable::Line12BelowItsComponents {
            line12: f.line12_all_mortgages,
            components,
        });
    }

    // ── Part I — Qualified Loan Limit ────────────────────────────────────────────────────────────

    // L1 — "Enter the average balance of all your grandfathered debt."
    let line1 = f.line1_grandfathered;
    // L2 — "Enter the average balance of all your home acquisition debt incurred after October 13,
    //       1987, and prior to December 16, 2017."
    let line2 = f.line2_acquisition_before_dec_16_2017;
    // L3 — "Enter $1,000,000 ($500,000 if married filing separately)"
    let line3 = line3_amount(status);
    // L4 — "Enter the larger of the amount on line 1 or the amount on line 3"
    let line4 = line1.max(line3);
    // L5 — "Add the amounts on lines 1 and 2. Enter the total here"
    let line5 = line1 + line2;
    // L6 — "Enter the smaller of the amount on line 4 or the amount on line 5"
    let line6 = line4.min(line5);

    // ── The branch after line 6, transcribed as a branch ─────────────────────────────────────────
    //
    //   • "If you have no home acquisition debt incurred after December 15, 2017, or the amount on
    //      line 6 is $750,000 ($375,000 if married filing separately) or more, line 6 is your
    //      qualified loan limit. Enter this amount on line 11 and go to Part II, line 12."
    //   • "If you have home acquisition debt incurred after December 15, 2017, go to line 7."
    //
    // ★★★ **The first limb's first clause is READ AS `line 7 == 0`, and that is a derivation, so it
    //     carries its equivalence proof here** (`CLAUDE.md` allows a closed form only with *"a
    //     written equivalence proof that names the branch where it breaks"* plus a KAT).
    //
    //     With line 7 = 0: line 10 = line 6 + 0 = line 6, and line 9 = max(line 6, line 8) ≥ line 6,
    //     so line 11 = min(line 9, line 10) = line 6 — **identically what the branch prescribes**.
    //     The equivalence is unconditional; it does not depend on the $750,000 clause, on the filing
    //     status, or on the sizes of lines 1 and 2.
    //
    //     ★ **Where it breaks:** nowhere for line 11, and exactly one place for the PRINTED sheet —
    //       a filer who HAS post-2017 acquisition debt whose average balance is zero (paid off in
    //       January, say) is sent to line 7 by the form and reaches line 11 through lines 7–10 with
    //       zeros in them, where this code leaves those four lines blank. Table 1 is a worksheet and
    //       btctax files no copy of it, so nothing the filer signs differs. Pinned by
    //       `tests::both_limbs_of_the_line_6_branch_agree_on_line_11`.
    //
    // ★ The first limb's SECOND clause is transcribed literally: `line6 >= line8_amount(status)`.
    let line8_figure = line8_amount(status);
    let (line7, line8, line9, line10, line11) = if line7_is_absent(f) || line6 >= line8_figure {
        (None, None, None, None, line6)
    } else {
        // L7 — "Enter the average balance of all your home acquisition debt incurred after
        //       December 15, 2017."
        let l7 = f.line7_acquisition_after_dec_15_2017;
        // L8 — "Enter $750,000 ($375,000 if married filing separately)"
        let l8 = line8_figure;
        // L9 — "Enter the larger of the amount on line 6 or the amount on line 8"
        let l9 = line6.max(l8);
        // L10 — "Add the amounts on lines 6 and 7. Enter the total here"
        let l10 = line6 + l7;
        // L11 — "Enter the smaller of line 9 or line 10. This is your qualified loan limit"
        let l11 = l9.min(l10);
        (Some(l7), Some(l8), Some(l9), Some(l10), l11)
    };

    // ── Part II — Deductible Home Mortgage Interest ──────────────────────────────────────────────

    // L12 — "Enter the total of the average balances of all mortgages from lines 1, 2, and 7 on all
    //        qualified homes."
    let line12 = f.line12_all_mortgages;

    //   • "If line 11 is less than line 12, go on to line 13."
    //   • "If line 11 is equal to or more than line 12, stop here. All of your interest on all the
    //      mortgages included on line 12 is deductible as home mortgage interest on
    //      Schedule A (Form 1040)."
    if line11 >= line12 {
        return Ok(Worked {
            sheet: Table1 {
                line1,
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
                line13: None,
                line14: None,
                line15: None,
                line16: None,
            },
            outcome: Outcome::AllDeductible,
        });
    }

    // L14 — "Divide the amount on line 11 by the amount on line 12. Enter the result as a decimal
    //        amount (rounded to three places)"
    let line14 = line11
        .checked_div(line12)
        .map(|q| q.round_dp_with_strategy(3, DOLLAR_ROUNDING))
        .ok_or(NotUsable::Line14DivisionFailed { line11, line12 })?;
    // L15 — "Multiply the amount on line 13 by the decimal amount on line 14. Enter the result.
    //        This is your deductible home mortgage interest."
    let line15 = line13 * line14;
    // L16 — "Subtract the amount on line 15 from the amount on line 13. Enter the result. This
    //        isn't home mortgage interest."
    let line16 = line13 - line15;
    Ok(Worked {
        sheet: Table1 {
            line1,
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
            line13: Some(line13),
            line14: Some(line14),
            line15: Some(line15),
            line16: Some(line16),
        },
        outcome: Outcome::Limited {
            ratio: line14,
            deductible: line15,
            not_home_mortgage_interest: line16,
        },
    })
}

/// The first clause of Part I's first branch bullet — *"If you have no home acquisition debt
/// incurred after December 15, 2017"* — read as line 7 being zero. See the equivalence proof beside
/// the branch in [`figure`].
fn line7_is_absent(f: &Table1Facts) -> bool {
    f.line7_acquisition_after_dec_15_2017 == Usd::ZERO
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Where line 15 lands: the Schedule A apportionment the instructions prescribe.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **How line 15 splits across Schedule A lines 8a, 8b and 8c.**
///
/// ★★★ Line 15's own sentence is *"Enter this amount on Schedule A (Form 1040)"* — with no line
/// number — because Table 1 spans THREE Schedule A lines. Table 2, *Where To Deduct Your Interest
/// Expense*, names them:
///
/// | IF you have … | THEN deduct it on … |
/// |---|---|
/// | deductible home mortgage interest and points reported on Form 1098 | Schedule A (Form 1040), line 8a |
/// | deductible home mortgage interest not reported on Form 1098 | Schedule A (Form 1040), line 8b |
/// | deductible points not reported on Form 1098 | Schedule A (Form 1040), line 8c |
///
/// ★★★ **And the rule for splitting is the publication's own, not an invention.** *Claiming your
/// deductible points*, item 2: *"Multiply the amount in item 1 by the decimal amount on line 14.
/// Enter the result on Schedule A (Form 1040), line 8a or 8c, whichever applies. This amount is
/// fully deductible."* So each component is multiplied by line 14 and entered on its own line —
/// which is also the only split under which the three printed lines sum to line 15 plus the
/// apportioned points.
///
/// ★ **Points are NOT in line 13** (*"Don't include points or mortgage insurance premiums on this
/// line"*), which is exactly why they need this separate path: they are multiplied by the same line
/// 14 and never pass through lines 13, 15 or 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Apportionment {
    /// Schedule A **line 8a** — the Form 1098 interest and points, times line 14.
    pub line8a: Usd,
    /// Schedule A **line 8b** — the interest not reported on a Form 1098, times line 14.
    pub line8b: Usd,
    /// Schedule A **line 8c** — the points not reported on a Form 1098, times line 14.
    pub line8c: Usd,
    /// Line 14 itself (or `1` on the STOP), carried so a caller can show the filer the fraction.
    pub ratio: Usd,
}

/// Apply a worked Table 1 to the three Schedule A components.
///
/// ★ Each argument is the UNLIMITED figure the line would print without Table 1, so a `Worked` whose
/// outcome is [`Outcome::AllDeductible`] returns them unchanged — *"All of your interest on all the
/// mortgages included on line 12 is deductible"*.
#[must_use]
pub fn apportion(
    worked: &Worked,
    form_1098_interest_and_points: Usd,
    interest_not_on_1098: Usd,
    points_not_on_1098: Usd,
) -> Apportionment {
    let ratio = worked.ratio();
    Apportionment {
        line8a: form_1098_interest_and_points * ratio,
        line8b: interest_not_on_1098 * ratio,
        line8c: points_not_on_1098 * ratio,
        ratio,
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The return's own readers — ONE definition each, shared by the screen and by the printed form.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **Worksheet line 13, off the return** — *"the total amount of interest that you paid on the loans
/// from line 12"*.
///
/// The line 13 instructions name two components and this sums exactly those two:
///
/// - *"you should get Form 1098 … This form will show the amount of interest to enter on line 13"* ⇒
///   Σ **box 1** across every transcribed Form 1098 row;
/// - *"Also, include on this line any other interest payments made on debts secured by a qualified
///   home for which you didn't receive a Form 1098"* ⇒ the line-8b rows.
///
/// ★ **Box 6 (points) is deliberately excluded**: *"Don't include points or mortgage insurance
/// premiums on this line."* Points reach Schedule A through [`apportion`] instead, multiplied by the
/// same line 14, exactly as *Claiming your deductible points* prescribes.
///
/// ★★ **DERIVED, not collected a second time.** Line 13 names precisely two things btctax already
/// holds row by row, so a `line13` field beside them would be a second testimony about one figure —
/// the shape `TwoTestimoniesAboutStateRefund` exists to refuse.
#[must_use]
pub fn line13_interest_paid(ri: &crate::tax::return_inputs::ReturnInputs) -> Usd {
    let on_1098: Usd = ri.form_1098.iter().map(|r| r.box1_interest).sum();
    on_1098 + ri.mortgage_interest_not_on_1098_total()
}

/// **Work Table 1 for this return** — the ONE definition, read by both the refusal screen and the
/// printed Schedule A.
///
/// ★★★ *"Two chains ⇒ the test IS the comparison"*, and the cheaper repair is to have one chain. The
/// refusal that stands in front of Schedule A line 8a and the figure that prints on it must agree by
/// construction, not because two call sites happen to pass the same arguments.
pub fn decide(ri: &crate::tax::return_inputs::ReturnInputs) -> Result<Worked, NotUsable> {
    figure(
        ri.filing_status,
        ri.schedule_a
            .as_ref()
            .and_then(|a| a.pub936_table1.as_ref()),
        line13_interest_paid(ri),
    )
}

/// **Whether this return's Schedule A must be limited by Table 1 at all.**
///
/// `true` iff the filer declared that one of the §163(h)(3)(B) limits applies
/// ([`ScheduleAInputs::mortgage_within_debt_limit`](crate::tax::return_inputs::ScheduleAInputs::mortgage_within_debt_limit)
/// `== Some(false)`) on a return where that question is LIVE.
///
/// ★ Read through [`question_is_live`](crate::tax::questions::question_is_live) rather than
/// re-typing the liveness conjuncts, so this cannot drift from the question that collects the answer
/// (T9's defect: *"three new rules each re-typing a liveness conjunct"*).
#[must_use]
pub fn limit_is_declared(ri: &crate::tax::return_inputs::ReturnInputs) -> bool {
    crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::MortgageWithinDebtLimit,
        ri,
    ) && ri
        .schedule_a
        .as_ref()
        .is_some_and(|a| a.mortgage_within_debt_limit == Some(false))
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

    /// **This module's own source**, so the sixteen line texts are checked where they are WRITTEN.
    ///
    /// ★★★ The alternative — a second `&'static str` table beside the doc comments — is the defect
    /// this repo names *"two chains ⇒ the test IS the comparison"*: two copies drift, and the one a
    /// reader trusts is the doc comment while the one the test checks is the table. There is exactly
    /// one copy of each line's text in this file, and [`line_docs`] reads it.
    const SELF_SOURCE: &str = include_str!("pub936_table1.rs");

    /// Collapse every run of whitespace to a single space.
    ///
    /// `pdftotext -layout` wraps a clause mid-sentence and lays the worksheet's answer-box column
    /// beside its label column, so nothing here can be compared line-for-line. Whitespace is the only
    /// thing normalised: every other glyph must match, apostrophes and dashes included.
    fn norm(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// [`norm`], then **hyphens and spaces removed**.
    ///
    /// ★★★ **ONLY for the two-column pages, and the blind spot is named because it is real.**
    /// `pdftotext` breaks a word across a line with a soft hyphen (*"after Octo- ber 13, 1987"*) and
    /// the text layer cannot distinguish that from a real hyphen in *"mixed-use"*. Removing hyphens
    /// AND spaces from **both sides** makes the comparison insensitive to exactly that artifact and
    /// to nothing else: every letter, digit, apostrophe, bracket and full stop must still match, in
    /// order.
    ///
    /// ★ The worksheet block itself has **no** hyphen breaks (asserted by
    /// [`the_worksheet_block_has_no_soft_hyphens`]), so the sixteen line texts and the four branches
    /// are checked under plain [`norm`] — the stronger comparison — and only the Table 1 Instructions'
    /// prose needs this one.
    fn norm_wordwise(s: &str) -> String {
        norm(s).replace(['-', ' '], "")
    }

    /// Every archived revision of Pub. 936, as `(file name, text)`.
    ///
    /// ★★★ **Read out of the DIRECTORY, never from a list of file names typed here** (`CLAUDE.md`'s
    /// highest-yield rule). Archiving a later revision pulls it into every assertion below with no
    /// edit to this file — which is the whole point, because the thing a hand list cannot know is
    /// that its set grew.
    fn archived_revisions() -> Vec<(String, String)> {
        let dir = repo_root().join(EXTRACT_DIR);
        let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .map(|e| e.expect("a readable directory entry").path())
            .filter_map(|p| {
                let name = p.file_name()?.to_str()?.to_string();
                if !name.starts_with(EXTRACT_PREFIX) || !name.ends_with(".txt") {
                    return None;
                }
                let text = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
                Some((name, text))
            })
            .collect();
        out.sort();
        assert!(
            !out.is_empty(),
            "no archived Pub. 936 extract found under {} — everything below would then be measuring \
             nothing",
            dir.display()
        );
        out
    }

    /// The revision named by [`EXTRACT_PATH`], which is the one this module was typed from.
    fn primary_extract() -> String {
        let p = repo_root().join(EXTRACT_PATH);
        std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
    }

    /// The Table 1 worksheet's own PHYSICAL lines: from its title through line 16, exclusive of the
    /// page footer that ends the page.
    ///
    /// ★ The end is DERIVED (the first footer line after the title), not a hand-typed line number.
    fn table1_physical_lines(text: &str) -> Vec<&str> {
        let all: Vec<&str> = text.lines().collect();
        let start = all
            .iter()
            .position(|l| {
                l.trim_start_matches('\u{c}')
                    .starts_with("Table 1. Worksheet To Figure Your Qualified Loan Limit")
            })
            .expect("the extract prints Table 1's title");
        let end = all
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, l)| is_page_footer(l))
            .map(|(i, _)| i)
            .expect("Table 1's page ends with a footer");
        all[start..end].to_vec()
    }

    /// Whether a physical line is nothing but this publication's page footer.
    ///
    /// The footer prints as *"Publication 936 (2025)    13"* on one page and *"14
    /// Publication 936 (2025)"* on the next, so both orders are recognised. It is not part of the
    /// text, and leaving it in splices a page number into the middle of a sentence that crosses a
    /// column break — which is exactly what happened while this was being written: the Line 2
    /// instruction read *"…and enter the total on **16** line 2. Include…"*.
    fn is_page_footer(line: &str) -> bool {
        let t = line.trim();
        let rest = t.strip_prefix("Publication 936 (").or_else(|| {
            t.split_once(' ')
                .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .map(|(_, r)| r.trim())
                .and_then(|r| r.strip_prefix("Publication 936 ("))
        });
        let Some(rest) = rest else { return false };
        let Some((year, tail)) = rest.split_once(')') else {
            return false;
        };
        year.len() == 4
            && year.chars().all(|c| c.is_ascii_digit())
            && tail.trim().chars().all(|c| c.is_ascii_digit())
    }

    /// The Table 1 worksheet block, normalised, with **one documented repair to the text layer**.
    ///
    /// ★★★ **The repair, and its exact scope: one token.** The second Part I branch bullet reads
    /// *"• If you have home acquisition debt incurred after December 15, 2017, go to line 7."* on the
    /// page, and `pdftotext` emits the leading `If` on the line AFTER the bullet it begins — so the
    /// raw text layer says *"• you have home acquisition debt … go to line 7. If"*. A physical line
    /// whose entire content is the word `If` is therefore moved to the front of the nearest preceding
    /// bullet. Nothing else is reordered, and
    /// [`the_if_repair_moves_exactly_one_token_and_invents_nothing`] pins that.
    fn table1_block(text: &str) -> String {
        let mut out: Vec<String> = Vec::new();
        for l in table1_physical_lines(text) {
            if l.trim() == "If" {
                if let Some(b) = out
                    .iter()
                    .rposition(|s| s.trim_start().starts_with('\u{2022}'))
                {
                    out[b] = out[b].replacen('\u{2022}', "\u{2022} If", 1);
                    continue;
                }
            }
            out.push(l.to_string());
        }
        norm(&out.join("\n"))
    }

    /// The column at which a page's RIGHT column begins, or `None` when the page is single-column.
    ///
    /// ★★★ **DERIVED from the page, never a typed offset.** Pub. 936's gutter is not at the same
    /// column on every page — it is 61 on the page carrying the Table 1 Instructions' exit and 64 on
    /// the page carrying the per-line instructions — so a constant would silently mis-split one of
    /// them. This finds the **widest run of columns that is blank on every line of the page**, which
    /// is what a gutter is, and requires real content on both sides of it so a single-column page
    /// (the worksheet itself) is recognised as one.
    fn gutter(page: &[&str]) -> Option<usize> {
        let chars: Vec<Vec<char>> = page.iter().map(|l| l.chars().collect()).collect();
        let width = chars.iter().map(Vec::len).max().unwrap_or(0);
        if width < 60 {
            return None;
        }
        let blank: Vec<bool> = (0..width)
            .map(|c| chars.iter().all(|l| l.get(c).is_none_or(|ch| *ch == ' ')))
            .collect();
        let nonblank = |from: usize, to: usize| {
            chars
                .iter()
                .filter(|l| {
                    l.iter()
                        .skip(from)
                        .take(to - from)
                        .any(|c| !c.is_whitespace())
                })
                .count()
        };
        let mut best: Option<(usize, usize)> = None;
        let mut c = 30;
        while c < width {
            if !blank[c] {
                c += 1;
                continue;
            }
            let s = c;
            while c < width && blank[c] {
                c += 1;
            }
            if c - s < 3 || c >= width {
                continue;
            }
            if nonblank(0, s) >= 3
                && nonblank(c, width) >= 3
                && best.is_none_or(|(a, b)| c - s > b - a)
            {
                best = Some((s, c));
            }
        }
        best.map(|(_, end)| end)
    }

    /// One page's lines with the footers dropped.
    fn page_body(page: &str) -> Vec<&str> {
        page.lines().filter(|l| !is_page_footer(l)).collect()
    }

    /// A page's two columns (or its single column), each as owned lines.
    fn columns(lines: &[&str]) -> Vec<Vec<String>> {
        let cut = |l: &str, from: usize, to: Option<usize>| -> String {
            let it = l.chars().skip(from);
            match to {
                Some(t) => it.take(t - from).collect(),
                None => it.collect(),
            }
        };
        match gutter(lines) {
            None => vec![lines.iter().map(|l| (*l).to_string()).collect()],
            Some(off) => vec![
                lines.iter().map(|l| cut(l, 0, Some(off))).collect(),
                lines.iter().map(|l| cut(l, off, None)).collect(),
            ],
        }
    }

    /// The whole publication in READING ORDER: page by page, left column then right column, with the
    /// page footers dropped — then [`norm_wordwise`]d.
    ///
    /// ★ Pages are separated by `\u{c}` (the form feed `pdftotext` emits), and a `|` is inserted
    ///   between them so a quote cannot match across a page boundary. Within a page the two columns
    ///   ARE joined, because that is the reading order and the Line 2 instruction genuinely spans the
    ///   break.
    fn reading_order(text: &str) -> String {
        let mut out: Vec<String> = Vec::new();
        for page in text.split('\u{c}') {
            let lines = page_body(page);
            let joined: Vec<String> = columns(&lines)
                .iter()
                .map(|col| norm(&col.join("\n")))
                .collect();
            out.push(joined.join(" "));
        }
        norm_wordwise(&out.join(" | "))
    }

    /// The sixteen numbered lines' doc comments, parsed out of [`SELF_SOURCE`]'s `Table1` struct:
    /// `(line number, the first quoted span of the field's doc block)`.
    fn line_docs() -> Vec<(u8, String)> {
        let body = SELF_SOURCE
            .split_once("pub struct Table1 {")
            .expect("this file declares `pub struct Table1`")
            .1
            .split_once("\n}")
            .expect("the struct closes")
            .0;
        let mut out: Vec<(u8, String)> = Vec::new();
        let mut doc: Vec<&str> = Vec::new();
        for raw in body.lines() {
            let t = raw.trim();
            if let Some(d) = t.strip_prefix("///") {
                doc.push(d.trim());
                continue;
            }
            if let Some((num, _)) = t.strip_prefix("pub line").and_then(|r| r.split_once(':')) {
                let n: u8 = num.parse().expect("a `pub lineN:` field name");
                let joined = doc.join(" ");
                let (_, after) = joined.split_once('"').unwrap_or_else(|| {
                    panic!("line {n}'s doc comment must quote the form's own instruction text")
                });
                let (quote, _) = after
                    .split_once('"')
                    .unwrap_or_else(|| panic!("line {n}'s quote is not closed"));
                out.push((n, norm(quote)));
            }
            if !t.is_empty() {
                doc.clear();
            }
        }
        out
    }

    /// `figure`'s body with the comment markers stripped, so the branch narration can be compared
    /// against the checked strings.
    ///
    /// ★ The `//` must come off before the comparison: a normalised comment block reads
    ///   *"…or the amount on **//** line 6 is $750,000…"*, which matches nothing. (It reds correctly
    ///   rather than passing falsely, and that is how this was found.)
    fn figure_body() -> String {
        let body = SELF_SOURCE
            .split_once("pub fn figure(")
            .expect("this file declares `pub fn figure`")
            .1
            .split_once("\n}\n")
            .expect("`figure` closes")
            .0;
        norm(
            &body
                .lines()
                .map(|l| l.trim().trim_start_matches('/').trim())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    /// Assert `quote` appears in `hay` verbatim AND is not a TRUNCATION of a longer sentence: on the
    /// worksheet every instruction runs into the dot leaders that reach its answer box, so anything
    /// else following the match means the quote stopped early.
    fn assert_runs_to_the_answer_box(hay: &str, label: &str, quote: &str) {
        let at = hay
            .find(quote)
            .unwrap_or_else(|| panic!("{label}: not verbatim in the extract.\n  wanted: {quote}"));
        let rest = &hay[at + quote.len()..];
        assert!(
            rest.starts_with(" ."),
            "{label}: the quote does not end where the form ends the instruction — it is followed \
             by {:?} rather than the dot leaders that run to the answer box. A TRUNCATED citation is \
             a substring of the real one.",
            &rest[..rest.len().min(48)]
        );
    }

    /// Assert `quote` appears verbatim and ends at a sentence boundary (the next token starts a new
    /// bullet, a numbered line, or a capitalised sentence).
    fn assert_whole_sentence(hay: &str, label: &str, quote: &str) {
        let at = hay
            .find(quote)
            .unwrap_or_else(|| panic!("{label}: not verbatim in the extract.\n  wanted: {quote}"));
        let next = hay[at + quote.len()..].trim_start();
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
            "{label}: the quote ends mid-clause — the next token is {:?}, which begins lowercase, so \
             it is TRUNCATED.",
            &next[..next.len().min(48)]
        );
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // CONFORMANCE — is every line present, and does each doc comment match the official text?
    // ════════════════════════════════════════════════════════════════════════════════════════════

    #[test]
    fn the_worksheet_block_has_no_soft_hyphens() {
        // ★ The premise of checking the sixteen line texts under plain `norm` rather than
        //   `norm_wordwise`: the worksheet's lines are short enough that `pdftotext` never breaks a
        //   word across one. If a future revision reflows it, this reds and the doc comments are then
        //   checked under the weaker comparison deliberately rather than by accident.
        for (name, text) in archived_revisions() {
            for l in table1_physical_lines(&text) {
                assert!(
                    !l.trim_end().ends_with('-'),
                    "{name}: Table 1's block now breaks a word across a line ({l:?}), so the \
                     verbatim check below needs `norm_wordwise` and its blind spot"
                );
            }
        }
    }

    #[test]
    fn every_numbered_line_doc_comment_is_verbatim_in_the_extract() {
        let docs = line_docs();
        assert_eq!(docs.len(), 16, "Table 1 has sixteen numbered lines");
        for (name, text) in archived_revisions() {
            let block = table1_block(&text);
            for (n, quote) in &docs {
                assert_runs_to_the_answer_box(&block, &format!("{name} line {n}"), quote);
            }
        }
    }

    /// Every line number Table 1 prints, read off the block: a physical line that begins with `N.`
    /// followed by whitespace is a numbered instruction; a continuation line never does.
    fn printed_line_numbers(text: &str) -> BTreeSet<u8> {
        table1_physical_lines(text)
            .iter()
            .filter_map(|l| {
                let t = l.trim_start();
                let (n, rest) = t.split_once('.')?;
                let n: u8 = n.parse().ok()?;
                rest.starts_with(char::is_whitespace).then_some(n)
            })
            .collect()
    }

    #[test]
    fn the_line_numbers_are_read_off_the_form() {
        // ★★★ **The expected set is DERIVED from the extract, not a range and not a hand list** —
        //     `CLAUDE.md`: *"a conformance KAT must enumerate the expected line set FROM the form's
        //     extracted text, never from a range or a hand-written list"*. The `BTreeSet` built from
        //     `1..=38` when the label set was 48 is the defect this shape avoids.
        let transcribed: BTreeSet<u8> = line_docs().into_iter().map(|(n, _)| n).collect();
        for (name, text) in archived_revisions() {
            let printed = printed_line_numbers(&text);
            assert_eq!(
                printed, transcribed,
                "{name}: the line numbers Table 1 PRINTS and the ones this module transcribes must \
                 be the same set (left: the form, right: this module)"
            );
        }
    }

    #[test]
    fn every_branch_and_stop_is_verbatim_in_the_extract() {
        for (name, text) in archived_revisions() {
            let block = table1_block(&text);
            for b in BRANCHES {
                assert_whole_sentence(&block, &format!("{name}: a branch bullet"), b);
            }
        }
    }

    #[test]
    fn every_branch_text_appears_verbatim_in_figures_own_comments() {
        // ★★★ **THE ANTI-DRIFT HALF.** `figure` narrates each branch inline, beside the code that
        //     implements it, because that is where a reader checks the logic. The `const`s above are
        //     what the extract is checked against. This is what stops the two from becoming two
        //     different sentences: edit either and this reds.
        let body = figure_body();
        for b in BRANCHES {
            assert!(
                body.contains(&norm(b)),
                "`figure` no longer narrates this branch in the form's own words, so its inline \
                 comment and the checked constant have drifted:\n  {b}"
            );
        }
    }

    #[test]
    fn every_instruction_quote_is_verbatim_in_the_extract() {
        for (name, text) in archived_revisions() {
            let page = reading_order(&text);
            let check = |label: String, q: &str| {
                assert!(
                    page.contains(&norm_wordwise(q)),
                    "{label}: not verbatim in the extract.\n  wanted: {q}"
                );
            };
            for (n, q) in LINE_INSTRUCTIONS {
                check(format!("{name}: the line {n} instructions"), q);
            }
            check(format!("{name}: line 12's Note"), LINE_12_NOTE);
            check(
                format!("{name}: Claiming your deductible points, item 2"),
                POINTS_TIMES_LINE_14,
            );
            for q in NO_TABLE_1_NEEDED {
                check(format!("{name}: the \"you don't need Table 1\" exit"), q);
            }
        }
    }

    #[test]
    fn the_instructed_line_set_is_read_off_the_form() {
        // ★ DERIVED again: the Table 1 Instructions print a `Line N` heading for exactly the lines
        //   they instruct, and `LINE_INSTRUCTIONS` must name that set — no more (a quote for a
        //   heading the form does not print) and no fewer (a heading nobody transcribed).
        let transcribed: BTreeSet<u8> = LINE_INSTRUCTIONS.iter().map(|(n, _)| *n).collect();
        for (name, text) in archived_revisions() {
            let mut printed: BTreeSet<u8> = BTreeSet::new();
            for page in text.split('\u{c}') {
                let lines = page_body(page);
                for col in columns(&lines) {
                    for l in col {
                        if let Some(n) = l.trim().strip_prefix("Line ") {
                            if let Ok(n) = n.parse::<u8>() {
                                printed.insert(n);
                            }
                        }
                    }
                }
            }
            assert_eq!(
                printed, transcribed,
                "{name}: the Table 1 Instructions print a heading for these lines and \
                 `LINE_INSTRUCTIONS` must name those (left: the form, right: this module)"
            );
        }
    }

    #[test]
    fn table_2_routes_each_component_to_the_schedule_a_line_this_module_uses() {
        // ★★★ **`apportion`'s three destinations, read off Table 2 rather than asserted.** The rows
        //     that distinguish 8a from 8b differ only in their SECOND physical line ("and points
        //     reported on Form 1098" versus "not reported on Form 1098"), so both lines are checked —
        //     a check on the first alone would pass with the two swapped.
        let rows = [
            (
                "deductible home mortgage interest",
                "and points reported on Form 1098",
                "Schedule A (Form 1040), line 8a",
            ),
            (
                "deductible home mortgage interest",
                "not reported on Form 1098",
                "Schedule A (Form 1040), line 8b",
            ),
            (
                "deductible points not reported on",
                "Form 1098",
                "Schedule A (Form 1040), line 8c",
            ),
        ];
        for (name, text) in archived_revisions() {
            let lines: Vec<&str> = text.lines().collect();
            for (cell1, cell2, dest) in rows {
                let hit = lines
                    .windows(2)
                    .any(|w| w[0].contains(cell1) && w[0].contains(dest) && w[1].trim() == cell2);
                assert!(
                    hit,
                    "{name}: Table 2 must route {cell1:?} / {cell2:?} to {dest:?}"
                );
            }
        }
    }

    #[test]
    fn the_two_dollar_ceilings_are_the_ones_the_form_prints() {
        // ★ Both printed amounts, checked against the form's own sentences — and
        //   `line3_amount`/`line8_amount` checked to agree with them for EVERY filing status, so a
        //   status added to the enum cannot silently take a wrong ceiling.
        for (name, text) in archived_revisions() {
            let block = table1_block(&text);
            assert!(
                block.contains("Enter $1,000,000 ($500,000 if married filing separately)"),
                "{name}: line 3's printed amounts"
            );
            assert!(
                block.contains("Enter $750,000 ($375,000 if married filing separately)"),
                "{name}: line 8's printed amounts"
            );
        }
        for s in FilingStatus::ALL {
            let mfs = s == FilingStatus::Mfs;
            assert_eq!(
                line3_amount(s),
                if mfs { dec!(500000) } else { dec!(1000000) },
                "line 3 for {s:?}"
            );
            assert_eq!(
                line8_amount(s),
                if mfs { dec!(375000) } else { dec!(750000) },
                "line 8 for {s:?}"
            );
        }
    }

    #[test]
    fn every_archived_revision_is_a_publication_extract() {
        // ★★★ The BOUNDARY this module states instead of a revision table (see `EXTRACT_DIR`): every
        //     archived revision must print every sentence transcribed here, which the checks above
        //     enforce by looping `archived_revisions()`. This one asserts the LOOP is non-trivial —
        //     the thing a reader cannot see from a passing run, since an empty directory would make
        //     all of them vacuous.
        let revs = archived_revisions();
        assert!(
            revs.iter().any(|(n, _)| n.starts_with(EXTRACT_PREFIX)),
            "the archive must hold the revision this module was typed from"
        );
        for (name, text) in &revs {
            assert!(
                text.contains("For use in preparing"),
                "{name}: does not look like an IRS publication extract"
            );
        }
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // B1 — SEEN RED ONCE. Each of these plants the exact defect the check above exists to catch.
    //
    // ★ Per FR-235 the plants are NOT written in the checker's own vocabulary: each is a real
    //   mis-transcription of the kind a person makes (a paraphrase, a dropped tail, a wrong line
    //   number), fed to the same assertion the conformance tests use.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    #[test]
    #[should_panic(expected = "not verbatim in the extract")]
    fn a_paraphrased_line_is_rejected() {
        // Line 6 says "Enter the smaller of the amount on line 4 or the amount on line 5". This is
        // what a careful summariser writes, and it is not the form.
        assert_runs_to_the_answer_box(
            &table1_block(&primary_extract()),
            "planted paraphrase",
            "Enter the lesser of the amounts on lines 4 and 5",
        );
    }

    #[test]
    #[should_panic(expected = "TRUNCATED citation")]
    fn a_truncated_line_is_rejected() {
        // Line 11's real text ends "...This is your qualified loan limit". Dropping that sentence
        // loses the only place the worksheet NAMES its own output — and the remainder is a perfectly
        // plausible-looking quote.
        assert_runs_to_the_answer_box(
            &table1_block(&primary_extract()),
            "planted truncation",
            "Enter the smaller of line 9 or line 10",
        );
    }

    #[test]
    #[should_panic(expected = "not verbatim in the extract")]
    fn a_wrong_line_cross_reference_is_rejected() {
        // ★★★ **THE FORM 6251 LINE-33 DEFECT, planted here.** Line 16 subtracts line 15 from line
        //     **13**; reading a rendered page instead of the text layer once turned a `22` into a
        //     `12` on Form 6251 and inflated one vector's tentative minimum tax by $200,000. Here the
        //     same slip would make line 16 subtract from line 12 — a BALANCE — and print a nonsense
        //     figure that every other test would accept.
        assert_runs_to_the_answer_box(
            &table1_block(&primary_extract()),
            "planted cross-reference",
            "Subtract the amount on line 15 from the amount on line 12. Enter the result. This \
             isn't home mortgage interest. See the line 16 instructions",
        );
    }

    #[test]
    #[should_panic(expected = "be the same set")]
    fn dropping_a_line_reds_the_completeness_check() {
        // The completeness check's own kill: a transcription missing line 14 (the divide) still
        // computes something for every other line, and nothing else here would notice.
        let mut transcribed: BTreeSet<u8> = line_docs().into_iter().map(|(n, _)| n).collect();
        transcribed.remove(&14);
        let printed = printed_line_numbers(&primary_extract());
        assert_eq!(
            printed, transcribed,
            "the line numbers Table 1 PRINTS and the ones this module transcribes must be the same \
             set"
        );
    }

    #[test]
    #[should_panic(expected = "have drifted")]
    fn a_branch_comment_that_drifts_from_the_checked_text_reds() {
        // The anti-drift check's own kill: `figure`'s narration says "stop here" because the form
        // does, and a later editor tightening it to "stop" would leave the code describing an
        // instruction the form does not print.
        let body = figure_body();
        let drifted = BRANCH_STOP_ALL_DEDUCTIBLE.replace("stop here", "stop");
        assert!(
            body.contains(&norm(&drifted)),
            "`figure` no longer narrates this branch in the form's own words, so its inline comment \
             and the checked constant have drifted:\n  {drifted}"
        );
    }

    #[test]
    fn the_if_repair_moves_exactly_one_token_and_invents_nothing() {
        // ★★★ **The repair's own kill.** A reorder that can move any token could make an arbitrary
        //     quote match, which would turn the verbatim check into theatre. Two facts pin its scope:
        //     the raw block contains exactly ONE orphaned `If`, and the repaired block has the same
        //     multiset of words as the raw one.
        for (name, text) in archived_revisions() {
            let raw: Vec<&str> = table1_physical_lines(&text);
            let bare_ifs = raw.iter().filter(|l| l.trim() == "If").count();
            assert_eq!(
                bare_ifs, 1,
                "{name}: the text layer emits exactly one orphaned `If`; {bare_ifs} means the \
                 repair's premise has changed"
            );
            let raw_s = norm(&raw.join("\n"));
            let mut before: Vec<&str> = raw_s.split(' ').collect();
            let after_s = table1_block(&text);
            let mut after: Vec<&str> = after_s.split(' ').collect();
            before.sort_unstable();
            after.sort_unstable();
            assert_eq!(
                before, after,
                "{name}: the repair changed WHICH words the block contains, not merely their order"
            );
        }
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // KATs — the expected figures come from the PUBLICATION and from i1040sca, never from this code.
    //
    // ★★★ **THE ORACLES CANNOT SUBSTITUTE FOR ANY OF THIS.** Both consume Schedule A line 8a as an
    //     INPUT (§G-9), so their agreement says nothing about a single line below. And per FR-230 a
    //     vector whose expectation was computed with this implementation is worth nothing — so every
    //     asserted figure below is quoted, with its source, from an IRS document.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    /// Facts with every leaf named, so a field added to [`Table1Facts`] reds every vector until
    /// someone decides what it is here.
    fn facts(line1: Usd, line2: Usd, line7: Usd, line12: Usd) -> Table1Facts {
        Table1Facts {
            line1_grandfathered: line1,
            line2_acquisition_before_dec_16_2017: line2,
            line7_acquisition_after_dec_15_2017: line7,
            line12_all_mortgages: line12,
            mortgages_exceed_fair_market_value: false,
            april_2018_binding_contract: false,
            provenance: crate::tax::return_inputs::CarryProvenance::User,
        }
    }

    fn worked(status: FilingStatus, f: &Table1Facts, line13: Usd) -> Worked {
        figure(status, Some(f), line13).expect("this vector's worksheet is workable")
    }

    #[test]
    fn pub936s_own_worked_example_reaches_its_printed_line_15_and_line_16() {
        // ★★★ **THE PUBLICATION'S OWN VECTOR** — Pub. 936 (2025), the *Line 16* instructions'
        //     business-allocation Example
        //     (`legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt:1122-1148`).
        //     Every figure asserted below is printed in it:
        //
        //       "Mortgage A had an average balance of $90,000, and mortgage B had an average balance
        //        of $110,000."  and  "$200,000 (the total average balance of all mortgages)"
        //                                                          ⇒ line 12 = $200,000
        //       "You paid $14,000 of interest on mortgage A and $16,000 of interest on mortgage B."
        //       "the amount on line 13 (the $30,000 total interest paid)"
        //                                                          ⇒ line 13 = $30,000
        //       "You determine that $15,000 of the interest can be deducted as home mortgage
        //        interest."                                        ⇒ **line 15 = $15,000**
        //       "The amount on Table 1, line 16, of the worksheet ($15,000)"
        //                                                          ⇒ **line 16 = $15,000**
        //
        // ★★ **Part I's inputs are not in the example, and they are DETERMINED rather than guessed.**
        //    Line 14 = line 15 ÷ line 13 = 15,000 ÷ 30,000 = 0.500, and line 11 = line 14 × line 12 =
        //    0.500 × 200,000 = $100,000. So the publication's own three figures fix the qualified
        //    loan limit; the Part I facts below are simply *a* fact pattern that produces it
        //    ($100,000 of pre-2017 home acquisition debt, the rest of the balance being the home
        //    equity debt mortgage B's business proceeds are). What is ASSERTED is the publication's.
        let f = facts(Usd::ZERO, dec!(100000), Usd::ZERO, dec!(200000));
        let w = worked(FilingStatus::Single, &f, dec!(30000));

        // The publication's two inputs, as the worksheet's own lines.
        assert_eq!(
            w.sheet.line12,
            dec!(200000),
            "line 12 — the example's total average balance"
        );
        assert_eq!(
            w.sheet.line13,
            Some(dec!(30000)),
            "line 13 — $14,000 + $16,000"
        );
        // The back-solved limit, stated so a reader can check the division themselves.
        assert_eq!(w.sheet.line11, dec!(100000), "line 11 — 0.500 x $200,000");
        assert_eq!(
            w.sheet.line14,
            Some(dec!(0.500)),
            "line 14 — $100,000 / $200,000"
        );

        // ★★★ THE PUBLICATION'S OWN ANSWERS.
        assert_eq!(
            w.sheet.line15,
            Some(dec!(15000)),
            "★ line 15 — Pub. 936: \"You determine that $15,000 of the interest can be deducted as \
             home mortgage interest.\""
        );
        assert_eq!(
            w.sheet.line16,
            Some(dec!(15000)),
            "★ line 16 — Pub. 936: \"The amount on Table 1, line 16, of the worksheet ($15,000)\""
        );
        assert_eq!(
            w.outcome,
            Outcome::Limited {
                ratio: dec!(0.500),
                deductible: dec!(15000),
                not_home_mortgage_interest: dec!(15000),
            }
        );

        // ★ THE PREMISE THAT MAKES THIS VECTOR DISCRIMINATE, asserted so it cannot pass vacuously.
        //   Line 6 is the SMALLER of lines 4 and 5, and here those differ by $900,000 — so a
        //   transcription that took the larger would put line 11 at $1,000,000 and hand the filer the
        //   whole $30,000. Mutate line 6's `.min` to `.max` and this test reds.
        assert_ne!(
            w.sheet.line4, w.sheet.line5,
            "line 6's min/max must be observable here"
        );
        assert_eq!(w.sheet.line4, dec!(1000000));
        assert_eq!(w.sheet.line5, dec!(100000));
        assert_eq!(w.sheet.line6, dec!(100000), "the SMALLER of the two");
    }

    #[test]
    fn i1040sca_says_the_750000_limit_is_reduced_by_the_1000000_debt_and_line_11_agrees() {
        // ★★★ **A SECOND IRS DOCUMENT AS THE ORACLE.** i1040sca states the interaction in prose:
        //     *"If you also have qualifying debt subject to the $1,000,000 limitation … the $750,000
        //     limit for debt taken out after December 15, 2017, is reduced by the amount of your
        //     qualifying debt subject to the $1,000,000 limit."* (`i1040sca--2025.txt:1039-1046`.)
        //
        //     So for $400,000 of pre-2017 acquisition debt and $500,000 of post-2017 acquisition
        //     debt, the post-2017 ceiling is $750,000 − $400,000 = $350,000, and the qualified total
        //     is $400,000 + $350,000 = **$750,000**. Table 1's lines 9, 10 and 11 must produce that
        //     figure with the subtraction typed nowhere.
        let sca =
            std::fs::read_to_string(repo_root().join("design/forms/extract/i1040sca--2025.txt"))
                .expect("i1040sca--2025 is archived");
        assert!(
            norm(&sca).contains(
                "the $750,000 limit for debt taken out after December 15, 2017, is reduced by the \
                 amount of your qualifying debt subject to the $1,000,000 limit."
            ),
            "the sentence this vector's expectation comes from must still be in i1040sca"
        );

        let f = facts(Usd::ZERO, dec!(400000), dec!(500000), dec!(900000));
        let w = worked(FilingStatus::Mfj, &f, dec!(36000));
        assert_eq!(
            w.sheet.line6,
            dec!(400000),
            "the pre-2017 bucket, inside its $1,000,000 ceiling"
        );
        assert_eq!(w.sheet.line7, Some(dec!(500000)));
        assert_eq!(w.sheet.line8, Some(dec!(750000)));
        assert_eq!(
            w.sheet.line9,
            Some(dec!(750000)),
            "the LARGER of line 6 and line 8"
        );
        assert_eq!(w.sheet.line10, Some(dec!(900000)), "line 6 + line 7");
        assert_eq!(
            w.sheet.line11,
            dec!(750000),
            "★ i1040sca's own arithmetic: $400,000 of older debt + ($750,000 - $400,000) of newer = \
             $750,000"
        );
        assert_eq!(
            w.sheet.line14,
            Some(dec!(0.833)),
            "750000 / 900000 = 0.8333… ⇒ 0.833"
        );
    }

    #[test]
    fn grandfathered_debt_consumes_the_1000000_bucket_as_the_publication_says() {
        // ★★★ Pub. 936: *"The limits above are reduced (but not below zero) by the amount of your
        //     grandfathered debt"*, and *"Grandfathered debt isn't limited. All of the interest you
        //     paid on grandfathered debt is fully deductible home mortgage interest. However, the
        //     amount of your grandfathered debt reduces the limit for home acquisition debt."*
        //
        //     So a filer with $1,400,000 of grandfathered debt and $180,000 of post-2017 acquisition
        //     debt gets a qualified loan limit of exactly their grandfathered balance: the
        //     grandfathered debt is not limited, and it has consumed both ceilings. Line 11 == line 1,
        //     and none of the newer debt is inside the limit.
        let text = primary_extract();
        for sentence in [
            "The limits above are reduced (but not below zero) by the amount of your grandfathered \
             debt",
            "Grandfathered debt isn't limited.",
            "However, the amount of your grandfathered debt reduces the limit for home acquisition \
             debt.",
        ] {
            assert!(
                reading_order(&text).contains(&norm_wordwise(sentence)),
                "the sentence this vector's expectation comes from must still be in Pub. 936:\n  \
                 {sentence}"
            );
        }
        let f = facts(dec!(1400000), Usd::ZERO, dec!(180000), dec!(1580000));
        let w = worked(FilingStatus::Single, &f, dec!(60000));
        assert_eq!(
            w.sheet.line4,
            dec!(1400000),
            "line 4 — the LARGER of line 1 and $1,000,000"
        );
        assert_eq!(w.sheet.line6, dec!(1400000));
        assert_eq!(
            w.sheet.line11, w.sheet.line1,
            "★ the qualified loan limit IS the grandfathered balance — the newer debt gets none of it"
        );
        assert_eq!(
            w.sheet.line7, None,
            "…and the branch after line 6 skipped lines 7-10, because line 6 is over $750,000"
        );
    }

    #[test]
    fn the_line_6_branch_at_the_printed_750000_threshold_is_inclusive() {
        // ★★★ The branch's own words are the expectation: *"or the amount on line 6 is $750,000
        //     ($375,000 if married filing separately) **or more**, line 6 is your qualified loan
        //     limit. Enter this amount on line 11 and go to Part II, line 12."* — so at EXACTLY
        //     $750,000 the jump is taken and lines 7 through 10 stay blank.
        let f = facts(Usd::ZERO, dec!(750000), dec!(200000), dec!(950000));
        let w = worked(FilingStatus::Single, &f, dec!(40000));
        assert_eq!(w.sheet.line6, dec!(750000), "exactly the printed threshold");
        assert_eq!(
            w.sheet.line11,
            dec!(750000),
            "line 6 IS the qualified loan limit"
        );
        for (n, v) in [
            (7, w.sheet.line7),
            (8, w.sheet.line8),
            (9, w.sheet.line9),
            (10, w.sheet.line10),
        ] {
            assert_eq!(
                v, None,
                "line {n} is SKIPPED by the branch — blank, not -0-"
            );
        }
        // ★ A cent below the threshold takes the other limb, which is what makes "or more" a real
        //   boundary rather than an unexercised word.
        let g = facts(Usd::ZERO, dec!(749999.99), dec!(200000), dec!(950000));
        let below = worked(FilingStatus::Single, &g, dec!(40000));
        assert_eq!(
            below.sheet.line7,
            Some(dec!(200000)),
            "one cent under ⇒ go to line 7"
        );
    }

    #[test]
    fn the_line_12_stop_makes_all_interest_deductible() {
        // ★★★ *"If line 11 is equal to or more than line 12, stop here. All of your interest on all
        //     the mortgages included on line 12 is deductible as home mortgage interest on
        //     Schedule A (Form 1040)."* — so lines 13 through 16 are BLANK, not zero, and the
        //     apportionment leaves every component untouched.
        let f = facts(Usd::ZERO, dec!(600000), Usd::ZERO, dec!(600000));
        let w = worked(FilingStatus::Single, &f, dec!(24000));
        assert_eq!(w.outcome, Outcome::AllDeductible);
        for (n, v) in [
            (13, w.sheet.line13),
            (14, w.sheet.line14),
            (15, w.sheet.line15),
            (16, w.sheet.line16),
        ] {
            assert_eq!(v, None, "line {n} is SKIPPED by the STOP — blank, not -0-");
        }
        assert_eq!(
            w.ratio(),
            Usd::ONE,
            "\"ALL of your interest … is deductible\""
        );
        let ap = apportion(&w, dec!(24000), dec!(3000), dec!(500));
        assert_eq!(
            (ap.line8a, ap.line8b, ap.line8c),
            (dec!(24000), dec!(3000), dec!(500)),
            "every Schedule A component prints what it would have printed with no Table 1 at all"
        );
    }

    #[test]
    fn the_all_grandfathered_situation_reaches_the_line_12_stop() {
        // ★★★ The Table 1 Instructions' FIRST *"you don't need Table 1"* situation — *"All the
        //     mortgages are grandfathered debt."* — PROVED to be the line-12 STOP rather than
        //     asserted to be. This is why the module has no separate exit for it.
        let f = facts(dec!(2500000), Usd::ZERO, Usd::ZERO, dec!(2500000));
        let w = worked(FilingStatus::Mfs, &f, dec!(95000));
        assert_eq!(
            w.outcome,
            Outcome::AllDeductible,
            "\"All of the interest you paid on grandfathered debt is fully deductible\" — even at \
             $2.5M, and even for MFS, whose ceilings are halved"
        );
    }

    #[test]
    fn the_within_the_limits_situation_reaches_the_line_12_stop() {
        // ★★★ The Instructions' SECOND situation — *"The total of the mortgage balances for the
        //     entire year is within the limits discussed earlier under Home Acquisition Debt."*
        for status in FilingStatus::ALL {
            let inside = line8_amount(status);
            let f = facts(Usd::ZERO, Usd::ZERO, inside, inside);
            let w = worked(status, &f, dec!(30000));
            assert_eq!(
                w.outcome,
                Outcome::AllDeductible,
                "{status:?}: a balance exactly at the ceiling is INSIDE it"
            );
        }
    }

    #[test]
    fn mfs_halves_both_printed_ceilings_all_the_way_to_line_11() {
        // ★ The same facts under two filing statuses. $500,000 of post-2017 acquisition debt is
        //   inside the $750,000 ceiling and OVER the $375,000 MFS one, so the MFS filer's line 11 is
        //   capped where the single filer's is not — which is the halving doing work rather than
        //   merely being printed.
        let f = facts(Usd::ZERO, Usd::ZERO, dec!(500000), dec!(500000));
        assert_eq!(
            worked(FilingStatus::Single, &f, dec!(20000)).outcome,
            Outcome::AllDeductible,
            "$500,000 is inside the $750,000 ceiling"
        );
        let mfs = worked(FilingStatus::Mfs, &f, dec!(20000));
        assert_eq!(
            mfs.sheet.line11,
            dec!(375000),
            "★ MFS is capped at the halved ceiling"
        );
        assert_eq!(mfs.sheet.line14, Some(dec!(0.750)), "375000 / 500000");
        assert_eq!(
            mfs.outcome,
            Outcome::Limited {
                ratio: dec!(0.750),
                deductible: dec!(15000),
                not_home_mortgage_interest: dec!(5000),
            }
        );
    }

    #[test]
    fn both_limbs_of_the_line_6_branch_agree_on_line_11() {
        // ★★★ **THE EQUIVALENCE PROOF'S KAT** (`CLAUDE.md`: a derived form needs *"a written
        //     equivalence proof that names the branch where it breaks, and a KAT pinning that
        //     branch"*). `figure` reads *"If you have no home acquisition debt incurred after
        //     December 15, 2017"* as `line 7 == 0`; this walks that limb over a grid of Part I facts
        //     and asserts line 11 is exactly what lines 7-10 would have produced.
        for l1 in [Usd::ZERO, dec!(300000), dec!(1200000)] {
            for l2 in [Usd::ZERO, dec!(400000), dec!(900000)] {
                for status in FilingStatus::ALL {
                    let f = facts(l1, l2, Usd::ZERO, l1 + l2 + dec!(1));
                    let branch = worked(status, &f, dec!(10000));
                    // What the long way round WOULD have produced, from the same six lines.
                    let line6 = l1.max(line3_amount(status)).min(l1 + l2);
                    let line9 = line6.max(line8_amount(status));
                    let line10 = line6 + Usd::ZERO;
                    assert_eq!(
                        branch.sheet.line11,
                        line9.min(line10),
                        "line 1 = {l1}, line 2 = {l2}, {status:?}: the branch and lines 7-10 must \
                         agree on the qualified loan limit"
                    );
                    assert_eq!(
                        branch.sheet.line7, None,
                        "…and the branch leaves line 7 BLANK"
                    );
                }
            }
        }
    }

    #[test]
    fn a_zero_line_12_stops_before_the_division() {
        // ★ The module's claim that line 14 can never divide by zero, exercised rather than argued:
        //   line 11 ≥ 0 always, so line 12 = 0 ⇒ line 11 ≥ line 12 ⇒ the STOP.
        let f = facts(Usd::ZERO, Usd::ZERO, Usd::ZERO, Usd::ZERO);
        let w = worked(FilingStatus::Single, &f, Usd::ZERO);
        assert_eq!(w.outcome, Outcome::AllDeductible);
        assert_eq!(w.sheet.line14, None, "line 14 was never reached");
    }

    #[test]
    fn line_14_rounds_to_three_places_the_way_the_form_says() {
        // *"Enter the result as a decimal amount (rounded to three places)."* $1 in $3 is 0.3333…,
        // which the form prints as 0.333 — and line 15 is then 0.333 of line 13, not a third of it.
        let f = facts(Usd::ZERO, dec!(100000), Usd::ZERO, dec!(300000));
        let w = worked(FilingStatus::Single, &f, dec!(30000));
        assert_eq!(w.sheet.line14, Some(dec!(0.333)));
        assert_eq!(
            w.sheet.line15,
            Some(dec!(9990)),
            "$30,000 x 0.333, NOT $10,000"
        );
        assert_eq!(
            w.sheet.line16,
            Some(dec!(20010)),
            "line 13 - line 15 exactly"
        );
    }

    // ════════════════════════════════════════════════════════════════════════════════════════════
    // The refusals, and what still refuses after FR-200a.
    // ════════════════════════════════════════════════════════════════════════════════════════════

    #[test]
    fn no_facts_refuses_rather_than_deducting_the_full_amount() {
        assert_eq!(
            figure(FilingStatus::Single, None, dec!(30000)),
            Err(NotUsable::FactsNotCollected),
            "★ \"an entry is testimony\" — an unsupplied average balance is not zero, and a zero \
             line 12 would make every dollar deductible"
        );
    }

    #[test]
    fn the_two_limits_pub936_does_not_figure_each_refuse() {
        let mut f = facts(Usd::ZERO, dec!(900000), Usd::ZERO, dec!(900000));
        f.mortgages_exceed_fair_market_value = true;
        assert_eq!(
            figure(FilingStatus::Single, Some(&f), dec!(30000)),
            Err(NotUsable::FairMarketValueLimit)
        );
        let mut g = facts(Usd::ZERO, dec!(900000), Usd::ZERO, dec!(900000));
        g.april_2018_binding_contract = true;
        assert_eq!(
            figure(FilingStatus::Single, Some(&g), dec!(30000)),
            Err(NotUsable::April2018TransitionRule)
        );
    }

    #[test]
    fn pub936_2025_prints_no_fair_market_value_worksheet_which_is_why_that_limit_refuses() {
        // ★★★ **THE FINDING BEHIND `FairMarketValueLimit`, asserted rather than argued in prose.**
        //     i1040sca names four limits and sends all four to Pub. 936; Pub. 936 (2025) figures
        //     three. If a future revision reinstates the fourth, this reds — and the refusal should
        //     then become a transcription instead.
        let sca =
            std::fs::read_to_string(repo_root().join("design/forms/extract/i1040sca--2025.txt"))
                .expect("i1040sca--2025 is archived");
        assert!(
            norm(&sca).contains(
                "Limit when loans exceed the fair market value of the home. If the total amount of \
                 all mortgages is more than the fair market value of the home, see Pub. 936 to \
                 figure your deduction."
            ),
            "i1040sca must still name the fourth limit, or this refusal has no premise"
        );
        for (name, text) in archived_revisions() {
            assert!(
                !table1_block(&text).contains("fair market value"),
                "{name}: Table 1 now mentions the home's fair market value — TRANSCRIBE that line \
                 and retire `NotUsable::FairMarketValueLimit`"
            );
        }
    }

    #[test]
    fn a_line_12_below_its_own_components_refuses_rather_than_being_clamped() {
        // ★ The worksheet's own sentence makes line 12 the total of lines 1, 2 and 7. A figure below
        //   that sum would make line 14's denominator too small and the deduction too LARGE — the
        //   understatement direction — so it refuses.
        let f = facts(dec!(100000), dec!(200000), dec!(300000), dec!(500000));
        assert_eq!(
            figure(FilingStatus::Single, Some(&f), dec!(30000)),
            Err(NotUsable::Line12BelowItsComponents {
                line12: dec!(500000),
                components: dec!(600000),
            })
        );
        // …and exactly at the sum it is accepted, because that is the case the line text describes.
        let g = facts(dec!(100000), dec!(200000), dec!(300000), dec!(600000));
        assert!(figure(FilingStatus::Single, Some(&g), dec!(30000)).is_ok());
    }

    #[test]
    fn the_apportionment_multiplies_every_component_by_line_14() {
        // ★★★ *Claiming your deductible points*, item 2 — the publication's own rule, and the reason
        //     points (which line 13 EXCLUDES) still get limited.
        let f = facts(Usd::ZERO, dec!(500000), Usd::ZERO, dec!(1000000));
        let w = worked(FilingStatus::Single, &f, dec!(40000));
        assert_eq!(w.sheet.line14, Some(dec!(0.500)));
        let ap = apportion(&w, dec!(40000), dec!(6000), dec!(1200));
        assert_eq!(ap.line8a, dec!(20000));
        assert_eq!(ap.line8b, dec!(3000));
        assert_eq!(
            ap.line8c,
            dec!(600),
            "★ points are apportioned too, and never entered on line 13"
        );
        assert_eq!(ap.ratio, dec!(0.500));
    }
}
