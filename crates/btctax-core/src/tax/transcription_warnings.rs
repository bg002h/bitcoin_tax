//! ★★★ **R4 — THE CHECKS THE DOCUMENT ITSELF GUARANTEES, DISPLAYED AND NEVER WRITTEN.**
//!
//! R4: *"Checks the tool may run and **display**: W-2 box 4 ≈ 6.2% × box 3, box 6 ≈ 1.45% × box 5,
//! box 3 ≤ the wage base — **warnings**, typo detectors; the filer corrects the box, the tool never
//! writes it."*
//!
//! ## Why a warning and not a refusal, and why not a correction
//!
//! Each of these is arithmetic the ISSUER performed and printed. When a transcribed row breaks it,
//! the overwhelmingly likely cause is a **typing slip** — box 1 typed into box 3 — and the second
//! most likely is a genuinely unusual W-2 the tool has no business overruling (a corrected form, a
//! mid-year successor employer, a §3121 exemption). So:
//!
//! - **Never a refusal.** A refusal on a lawful edge case is a brick, and every one of these has an
//!   edge case. The figures still reach their lines exactly as transcribed.
//! - **Never a correction.** *The filer corrects the box.* A tool that "fixed" box 3 would be
//!   writing testimony the filer never gave — the whole class this codebase exists to refuse — and
//!   would make the printed return disagree with the paper it was copied from.
//!
//! ## The all-zero row
//!
//! R4's second warning has a different mechanism and is worth stating separately: *"a row whose
//! every income box is zero is most likely a row the filer began and did not finish — a payer issues
//! a 1099-INT at $10 or more."* The **threshold is the ISSUER'S**, printed in that form's own filing
//! instructions, which is why it applies to the information returns and NOT to the Form W-2: an
//! employer must issue a W-2 whatever the wages, so an all-zero W-2 is unremarkable and warning
//! about it would train the filer to ignore the whole class.
//!
//! ★ Nothing here reads [`crate::tax::tables::FullReturnParams`], and only the wage-base check reads
//!   the year's [`crate::tax::tables::TaxTable`] — passed as an `Option`, so the whole set runs while
//!   the filer is editing a year whose package has not arrived (R11).

use crate::conventions::Usd;
use crate::tax::return_inputs::ReturnInputs;
use rust_decimal_macros::dec;

/// §3101(a) — the employee OASDI rate box 4 withholds at.
const OASDI_RATE: Usd = dec!(0.062);
/// §3101(b)(1) — the employee Medicare rate box 6 withholds at.
const MEDICARE_RATE: Usd = dec!(0.0145);
/// §3101(b)(2) — Medicare plus the 0.9% Additional Medicare Tax, which is ALSO withheld into box 6.
///
/// ★ Used as an UPPER BOUND rather than as a second exact rule, so the check needs no $200,000
///   threshold of its own: the surtax applies only to the excess over that figure, so a box 6 above
///   `2.35% × box 5` cannot be explained by it at any wage.
const MEDICARE_PLUS_SURTAX_RATE: Usd = dec!(0.0235);

/// The slack a rounded figure is allowed before it is called a slip: one dollar, or a tenth of a
/// percent of the expected amount, whichever is larger.
///
/// ★ Deliberately loose. These exist to catch a figure in the WRONG BOX — an error of tens of
///   thousands — never a cent of rounding, and a check that cries on a rounding difference is a
///   check people learn to skip.
fn tolerance(expected: Usd) -> Usd {
    let proportional = expected.abs() * dec!(0.001);
    if proportional > dec!(1) {
        proportional
    } else {
        dec!(1)
    }
}

/// Which transcribed row a warning is about — the section a renderer must put the filer's cursor on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarnedDocument {
    W2,
    Form1099Int,
    Form1099Div,
    Form1099G,
    /// ★ T9 — the Form 1098 rows. The §163(h)(3)(B) ceiling warning is AGGREGATE, so it is attached
    ///   to the FIRST row: there is no single row it is about, and a warning attached to none would
    ///   be a warning no surface can place a cursor on.
    Form1098,
    Form1098E,
}

impl WarnedDocument {
    /// The document's IRS designation, for the message.
    #[must_use]
    pub const fn designation(self) -> &'static str {
        match self {
            WarnedDocument::W2 => "Form W-2",
            WarnedDocument::Form1099Int => "Form 1099-INT",
            WarnedDocument::Form1099Div => "Form 1099-DIV",
            WarnedDocument::Form1099G => "Form 1099-G",
            WarnedDocument::Form1098 => "Form 1098",
            WarnedDocument::Form1098E => "Form 1098-E",
        }
    }
}

/// One displayed warning about one transcribed row. It carries no severity and no remedy the tool
/// can apply: the only action is the filer re-reading the paper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionWarning {
    pub document: WarnedDocument,
    /// The row's index in its `Vec`, so a renderer can point at it. Zero-based; messages print it
    /// one-based, because the filer counts documents from one.
    pub row: usize,
    pub message: String,
}

/// ★★★ **Every transcription warning this return raises, in row order. NOTHING IS WRITTEN.**
///
/// `ss_wage_base` is the year's [`crate::tax::tables::TaxTable::ss_wage_base`], or `None` on a year
/// whose package has not arrived — the box-3 ceiling check is then simply not run, exactly like
/// every other params-gated rule (R11), rather than guessing a base.
#[must_use]
pub fn transcription_warnings(
    ri: &ReturnInputs,
    ss_wage_base: Option<Usd>,
    acquisition_debt_ceiling: Option<crate::tax::tables::AcquisitionDebtCeiling>,
) -> Vec<TranscriptionWarning> {
    let mut out = Vec::new();
    if let Some(message) = acquisition_debt_ceiling_warning(ri, acquisition_debt_ceiling) {
        out.push(TranscriptionWarning {
            document: WarnedDocument::Form1098,
            row: 0,
            message,
        });
    }
    let mut warn = |document: WarnedDocument, row: usize, message: String| {
        out.push(TranscriptionWarning {
            document,
            row,
            message,
        });
    };

    for (i, w) in ri.w2s.iter().enumerate() {
        let who = if w.employer.trim().is_empty() {
            format!("Form W-2 #{}", i + 1)
        } else {
            format!("the Form W-2 from {}", w.employer.trim())
        };
        // ★★★ SEAM REVIEW M-2 — THE ONE LAWFUL W-2 THESE TWO CHECKS FIRE ON, SUPPRESSED.
        //
        //     An employee whose wages were too small to withhold the Social Security / Medicare tax
        //     due on their reported TIPS gets a W-2 with box 4 below 6.2% of box 3 and/or box 6
        //     below 1.45% of box 5 — and the shortfall recorded in box 12 as code A or code B. The
        //     employer's own instructions say so in terms (`iw2w3--2026.txt:2412-2423`): code A is
        //     *"Uncollected social security or RRTA tax on tips … Do not include this amount in
        //     box 4"*, code B is *"Uncollected Medicare tax on tips … Do not include this amount in
        //     box 6"*. So the arithmetic is SUPPOSED to fall short by exactly that code's amount,
        //     and warning here trains the filer to ignore the class.
        //
        //     ★ Per BOX, not per W-2: code A excuses box 4 and code B excuses box 6, because that is
        //       what each instruction says. A W-2 with only code A whose box 6 is also wrong still
        //       warns about box 6 — a blanket "has A or B" suppression would lose that.
        let has_code = |c: &str| {
            w.box12
                .iter()
                .any(|e| e.code.trim().eq_ignore_ascii_case(c))
        };
        // ── box 4 ≈ 6.2% × box 3 ──────────────────────────────────────────────────────────────
        let expected4 = w.box3_ss_wages * OASDI_RATE;
        if !has_code("A") && (w.box4_ss_withheld - expected4).abs() > tolerance(expected4) {
            warn(
                WarnedDocument::W2,
                i,
                format!(
                    "{who}: box 4 (Social security tax withheld) is {}, but 6.2% of box 3 \
                     (Social security wages, {}) is {}. Employers withhold box 4 at exactly that \
                     rate, so one of the two boxes is probably mistyped — check the paper. btctax \
                     has changed nothing.",
                    money(w.box4_ss_withheld),
                    money(w.box3_ss_wages),
                    money(expected4)
                ),
            );
        }
        // ── box 6 between 1.45% and 2.35% of box 5 ────────────────────────────────────────────
        let low = w.box5_medicare_wages * MEDICARE_RATE;
        let high = w.box5_medicare_wages * MEDICARE_PLUS_SURTAX_RATE;
        if !has_code("B")
            && (w.box6_medicare_withheld < low - tolerance(low)
                || w.box6_medicare_withheld > high + tolerance(high))
        {
            warn(
                WarnedDocument::W2,
                i,
                format!(
                    "{who}: box 6 (Medicare tax withheld) is {}, and 1.45% of box 5 (Medicare \
                     wages and tips, {}) is {}. Box 6 also carries the 0.9% Additional Medicare \
                     Tax on wages over $200,000, so anything between {} and {} is ordinary — this \
                     is outside that. Check the paper; btctax has changed nothing.",
                    money(w.box6_medicare_withheld),
                    money(w.box5_medicare_wages),
                    money(low),
                    money(low),
                    money(high)
                ),
            );
        }
        // ── box 3 ≤ the year's Social Security wage base ──────────────────────────────────────
        if let Some(base) = ss_wage_base {
            if w.box3_ss_wages > base {
                warn(
                    WarnedDocument::W2,
                    i,
                    format!(
                        "{who}: box 3 (Social security wages) is {}, which is above this year's \
                         Social Security wage base of {}. No single employer reports more than the \
                         base in box 3 — box 1 is the usual figure typed here by mistake. Check the \
                         paper; btctax has changed nothing.",
                        money(w.box3_ss_wages),
                        money(base)
                    ),
                );
            }
        }
    }

    // ── The all-zero row, on the documents whose ISSUER has a reporting threshold. ────────────
    for (i, r) in ri.int_1099.iter().enumerate() {
        let income = r.box1_interest
            + r.box3_treasury_interest
            + r.box8_tax_exempt_interest
            + r.box10_market_discount;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099Int,
                i,
                all_zero(
                    WarnedDocument::Form1099Int,
                    i,
                    &r.payer,
                    "$10 or more of interest",
                ),
            );
        }
    }
    for (i, r) in ri.div_1099.iter().enumerate() {
        // ★ Seam review M-1: boxes 9 and 10 are income on this row too (they refuse rather than
        //   reach a line, but a row carrying one is NOT an all-zero row), so the "nothing here"
        //   warning must not fire on it — the refusal is the message that row gets.
        let income = r.box1a_ordinary
            + r.box2a_capgain_distr
            + r.box12_exempt_interest_dividends
            + r.box9_cash_liquidation
            + r.box10_noncash_liquidation;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099Div,
                i,
                all_zero(
                    WarnedDocument::Form1099Div,
                    i,
                    &r.payer,
                    "$10 or more of dividends",
                ),
            );
        }
    }
    for (i, r) in ri.g_1099.iter().enumerate() {
        // ★ Seam review M-1: boxes 5, 6, 7 and 9 join the income sum for the same reason as the
        //   1099-DIV row above — a row carrying one of them is not an all-zero row.
        let income = r.box1_unemployment
            + r.box2_state_refund
            + r.box5_rtaa_payments
            + r.box6_taxable_grants
            + r.box7_agriculture_payments
            + r.box9_market_gain
            + r.box10_family_leave_benefits;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099G,
                i,
                all_zero(
                    WarnedDocument::Form1099G,
                    i,
                    &r.payer,
                    "$10 or more of unemployment compensation, or any refund of state or local \
                     income tax",
                ),
            );
        }
    }
    for (i, r) in ri.form_1098e.iter().enumerate() {
        if r.box1_interest == Usd::ZERO {
            warn(
                WarnedDocument::Form1098E,
                i,
                all_zero(
                    WarnedDocument::Form1098E,
                    i,
                    &r.lender,
                    "$600 or more of student loan interest",
                ),
            );
        }
    }
    out
}

/// The all-zero-row message, one sentence per document, naming that issuer's own threshold.
fn all_zero(doc: WarnedDocument, i: usize, issuer: &str, threshold: &str) -> String {
    let named = if issuer.trim().is_empty() {
        format!("{} #{}", doc.designation(), i + 1)
    } else {
        format!("the {} from {}", doc.designation(), issuer.trim())
    };
    format!(
        "{named}: every income box on this row is zero. An issuer sends a {} only for {threshold}, \
         so a row with nothing on it is most likely one you began and did not finish — check the \
         paper, or remove the row. btctax has changed nothing.",
        doc.designation()
    )
}

/// Dollars, for a message a FILER reads: thousands-separated, cents only when there are any —
/// `$1,200,000`, `$6,200.35`. One line, because it delegates to the core-side house formatter
/// `crate::tax::advisories::fmt_usd`, which every advisory already prints money through.
///
/// ★★ **FR-118 — and this is deliberately NOT "the product's money formatter".** The warning it feeds
/// used to read *"adds up to $1200000.00, which is more than the $750000.00 the §163(h)(3)(B) limit
/// allows"*: the entire content of that sentence is a comparison of two seven-figure numbers, and an
/// ungrouped one has to be counted by eye. So prose gets grouped.
///
/// The OTHER formatter, `btctax_cli::render::fmt_money`, prints `1200000.00` ungrouped and **must stay
/// that way** — its module is *"Text rendering of CLI outputs … + FR10 CSV export"* and it imports
/// `csv::Writer`, so a thousands separator there would put a comma INSIDE an exported CSV field (or
/// silently force quoting, changing a contract that module says is stable). Corrupting an export to
/// tidy a warning is a poor trade. **The split is by AUDIENCE, not by oversight: prose a filer reads
/// is grouped, machine-readable output is not. Do not unify them.**
fn money(v: Usd) -> String {
    crate::tax::advisories::fmt_usd(v)
}

/// ★★★ **THE §163(h)(3)(B) ACQUISITION-DEBT CEILING CHECK — the ONE derivation** (R8 / T9).
///
/// **AGGREGATE, and that is the whole point.** *"For qualifying debt taken out after December 15,
/// 2017, you can only deduct home mortgage interest on up to $750,000 …"*
/// (`i1040sca--2025.txt:1040-1042`) — the limit is on the filer's home acquisition debt **counted
/// together**, so two mortgages at $500,000 each are over it while each row alone is silent. A
/// per-row check would never warn on exactly the household the limit was written for.
///
/// **Which ceiling** comes from box 3, the origination date, per row: a loan taken out on or before
/// 2017-12-15 is measured against $1,000,000 and a later one against $750,000, halved for MFS. Where
/// the rows straddle the date, the ceiling used is the **SMALLEST** any row would earn — the
/// instruction's own reduction rule (*"the $750,000 limit for debt taken out after December 15,
/// 2017, is reduced by the amount of your qualifying debt subject to the $1,000,000 limit"*,
/// `:1043-1046`) makes a mixed portfolio no more generous than the newer limit, and this is a
/// warning, so the conservative bound is the honest one.
///
/// ★★ **It writes nothing, and it is not a refusal.** Line 8a stays the filer's own
/// [`crate::tax::questions::QuestionId::MortgageWithinDebtLimit`] testimony, which refuses on `None`
/// and on `Some(false)`. The warning exists because that question used to be asked with **no
/// figure**: the filer was made to add up their own balances to answer it, and btctax was holding
/// box 2 the whole time.
///
/// ★ `None` for the ceiling is a year whose package has not arrived (R11): the check simply does not
///   run, exactly like every other params-gated rule, rather than guessing a limit.
///
/// ★★★ **It reads [`ReturnInputs::form_1098_deducted`], so it is SILENT for a standard-deduction
///   filer** (the T9 seam review's M-1). The warning's closing sentence tells the filer to *"check
///   the figure against the limit before you answer"* the `MortgageWithinDebtLimit` declaration —
///   a question that is not live for them and that they will never be asked. Pointing a filer at a
///   question the product never puts to them is worse than saying nothing.
#[must_use]
pub fn acquisition_debt_ceiling_warning(
    ri: &ReturnInputs,
    ceiling: Option<crate::tax::tables::AcquisitionDebtCeiling>,
) -> Option<String> {
    let ceiling = ceiling?;
    let deducted = ri.form_1098_deducted();
    if deducted.is_empty() {
        return None;
    }
    let total = ri.form_1098_outstanding_principal();
    // The smallest ceiling any transcribed row earns. `for_loan` already takes the stricter branch
    // for a row whose box 3 was never transcribed.
    let limit = deducted
        .iter()
        .map(|r| ceiling.for_loan(ri.filing_status, r.box3_origination_date))
        .min()?;
    if total <= limit {
        return None;
    }
    let rows = deducted.len();
    let plural = if rows == 1 { "" } else { "s" };
    // ★ N-1 (T9 seam review) — through the module's own `money()`, like every other warning here.
    //   This is the one warning whose whole content is a comparison of two large numbers, and raw
    //   interpolation printed it as `$900000`.
    let (total, limit) = (money(total), money(limit));
    Some(format!(
        "the outstanding mortgage principal in box 2 of the {rows} Form{plural} 1098 on this return \
         adds up to {total}, which is more than the {limit} the \u{a7}163(h)(3)(B) limit allows for \
         your filing status and origination date{s} \u{2014} \"Limits on home mortgage interest\" in the \
         Schedule A instructions. That does NOT mean line 8a is wrong: Pub. 936's Deductible Home \
         Mortgage Interest Worksheet figures how much of your interest is still deductible, and \
         btctax does not compute it. Check the figure against the limit before you answer \"were you \
         inside EVERY home-mortgage debt limit this year?\" \u{2014} btctax takes your answer and \
         changes nothing on its own",
        s = if rows == 1 { "" } else { "s" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{Form1098E, Form1099Div, Form1099G, Form1099Int, W2};
    use crate::tax::types::FilingStatus;
    use time::macros::date;

    // ── ★★★ R8 / T9 — the §163(h)(3)(B) ACQUISITION-DEBT CEILING WARNING. ──────────────────────

    /// TY2024's ceilings, transcribed once for these tests from the same four printed figures
    /// [`crate::tax::tables::AcquisitionDebtCeiling`] carries.
    fn ceilings() -> crate::tax::tables::AcquisitionDebtCeiling {
        crate::tax::tables::AcquisitionDebtCeiling {
            after_dec_15_2017: dec!(750000),
            after_dec_15_2017_mfs: dec!(375000),
            on_or_before_dec_15_2017: dec!(1000000),
            on_or_before_dec_15_2017_mfs: dec!(500000),
        }
    }

    /// One Form 1098 with `principal` in box 2 and `origination` in box 3.
    fn m1098(
        principal: Usd,
        origination: Option<time::Date>,
    ) -> crate::tax::return_inputs::Form1098 {
        crate::tax::return_inputs::Form1098 {
            lender: "Home Savings".into(),
            box1_interest: dec!(20000),
            box2_outstanding_principal: principal,
            box3_origination_date: origination,
            other_borrower_paid_interest: Some(false),
            ..Default::default()
        }
    }

    fn warned(
        rows: Vec<crate::tax::return_inputs::Form1098>,
        status: FilingStatus,
    ) -> Option<String> {
        let ri = ReturnInputs {
            filing_status: status,
            form_1098: rows,
            // ★★★ The ITEMIZE ELECTION, stated. The ceiling is a limit on a DEDUCTION, so the
            //     warning is silent without a Schedule A (the T9 seam review's M-1) — a fixture
            //     that left this `None` would be measuring the gate instead of the arithmetic, and
            //     `the_ceiling_warning_is_silent_for_a_standard_deduction_filer` measures the gate.
            schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs::default()),
            ..Default::default()
        };
        acquisition_debt_ceiling_warning(&ri, Some(ceilings()))
    }

    /// ★★★ **THE CEILING FIRES ON THE FIGURE, AND IS SILENT UNDER IT.**
    ///
    /// A 2019 loan is measured against $750,000: $900,000 warns, $700,000 does not. Both halves,
    /// because a check that warned on every 1098 would pass the first and be worthless.
    ///
    /// Mutation: change `total <= limit` to `total < limit` and the $750,000-exactly row reds;
    /// delete the comparison and the $700,000 row reds.
    #[test]
    fn the_acquisition_debt_warning_fires_over_the_ceiling_and_is_silent_under_it() {
        let d2019 = Some(date!(2019 - 06 - 01));
        assert!(
            warned(vec![m1098(dec!(900000), d2019)], FilingStatus::Single)
                .is_some_and(|w| w.contains("$900,000") && w.contains("$750,000")),
            "$900,000 of post-2017 debt is over the $750,000 ceiling and must name both figures"
        );
        assert_eq!(
            warned(vec![m1098(dec!(700000), d2019)], FilingStatus::Single),
            None,
            "$700,000 is under it and must be silent"
        );
        assert_eq!(
            warned(vec![m1098(dec!(750000), d2019)], FilingStatus::Single),
            None,
            "AT the ceiling is not over it — the instruction says \"up to $750,000\""
        );
    }

    /// ★★★ **IT IS AGGREGATE, AND THAT IS THE WHOLE POINT.**
    ///
    /// *"you can only deduct home mortgage interest on up to $750,000 … of that debt"* is a limit on
    /// the filer's acquisition debt COUNTED TOGETHER. Two mortgages at $500,000 each are over it
    /// while each row alone is silent — exactly the household the limit was written for, and exactly
    /// the household a per-row check would never warn.
    ///
    /// Mutation: sum a single row instead of `form_1098_outstanding_principal()` and the two-row
    /// assertion reds while the one-row assertion still passes.
    #[test]
    fn the_acquisition_debt_warning_sums_every_1098_row() {
        let d2019 = Some(date!(2019 - 06 - 01));
        assert_eq!(
            warned(vec![m1098(dec!(500000), d2019)], FilingStatus::Single),
            None,
            "one $500,000 mortgage is under the $750,000 ceiling"
        );
        assert!(
            warned(
                vec![m1098(dec!(500000), d2019), m1098(dec!(500000), d2019)],
                FilingStatus::Single
            )
            .is_some_and(|w| w.contains("$1,000,000")),
            "TWO of them are $1,000,000 of acquisition debt and must warn on the SUM"
        );
    }

    /// ★★★ **MFS IS HALVED, AND THE DATE SELECTS THE CEILING.**
    ///
    /// The instruction prints the married-filing-separately amount beside each limit — *"$750,000
    /// ($375,000 if you are married filing separately)"* — so a $400,000 balance warns an MFS filer
    /// and not a Single one. And a loan taken out ON OR BEFORE December 15, 2017 is measured against
    /// $1,000,000 instead, so the same $900,000 that warns a 2019 borrower is silent for a 2016 one.
    ///
    /// ★ A row whose box 3 was never transcribed takes the STRICTER later ceiling: an unknown date
    ///   can make the warning fire sooner, never later. Mutation: default `None` to the pre-2018
    ///   limit and the last assertion reds.
    #[test]
    fn the_acquisition_debt_warning_halves_for_mfs_and_reads_the_origination_date() {
        let d2019 = Some(date!(2019 - 06 - 01));
        let d2016 = Some(date!(2016 - 03 - 01));
        assert!(
            warned(vec![m1098(dec!(400000), d2019)], FilingStatus::Mfs).is_some(),
            "$400,000 is over the $375,000 MFS ceiling"
        );
        assert_eq!(
            warned(vec![m1098(dec!(400000), d2019)], FilingStatus::Single),
            None,
            "…and under the $750,000 unmarried one"
        );
        assert_eq!(
            warned(vec![m1098(dec!(900000), d2016)], FilingStatus::Single),
            None,
            "a loan taken out on or before December 15, 2017 is measured against $1,000,000"
        );
        assert!(
            warned(vec![m1098(dec!(900000), None)], FilingStatus::Single).is_some(),
            "an UNTRANSCRIBED box 3 takes the stricter post-2017 ceiling — fail-closed"
        );
        // The instruction's own boundary, to the day.
        assert_eq!(
            warned(
                vec![m1098(dec!(900000), Some(date!(2017 - 12 - 15)))],
                FilingStatus::Single
            ),
            None,
            "ON December 15, 2017 is \"on or before\" — the $1,000,000 ceiling"
        );
        assert!(
            warned(
                vec![m1098(dec!(900000), Some(date!(2017 - 12 - 16)))],
                FilingStatus::Single
            )
            .is_some(),
            "the NEXT day is \"after December 15, 2017\" — the $750,000 ceiling"
        );
    }

    /// ★★★ **THE WARNING WRITES NOTHING, AND THE DECLARATION STILL REFUSES.**
    ///
    /// R8: the ceiling is *"displayed as a warning beside the existing `MortgageWithinDebtLimit`
    /// declaration, which stays the filer's testimony"*. So an over-limit return with the question
    /// unanswered still refuses — the figure never answers it — and line 8a is untouched by the
    /// warning either way.
    ///
    /// ★ And on a year whose package has not arrived the check simply does not run (R11), rather
    ///   than guessing a limit.
    #[test]
    fn the_ceiling_warning_answers_nothing_and_waits_for_the_years_package() {
        use crate::tax::questions::QuestionId;
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            schedule_a: Some(crate::tax::return_inputs::ScheduleAInputs::default()),
            form_1098: vec![m1098(dec!(900000), Some(date!(2019 - 06 - 01)))],
            ..Default::default()
        };
        assert!(
            acquisition_debt_ceiling_warning(&ri, Some(ceilings())).is_some(),
            "the premise: this return is over the ceiling"
        );
        assert!(
            crate::tax::questions::question_is_live(QuestionId::MortgageWithinDebtLimit, &ri),
            "…and the declaration is live"
        );
        assert_eq!(
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_within_debt_limit),
            None,
            "★ THE KILL: the warning wrote nothing — the answer is still the filer's to give"
        );
        assert_eq!(
            acquisition_debt_ceiling_warning(&ri, None),
            None,
            "on a year with no package the check waits, exactly as every other params-gated rule does"
        );
        ri.form_1098.clear();
        assert_eq!(
            acquisition_debt_ceiling_warning(&ri, Some(ceilings())),
            None,
            "and a return with no Form 1098 has no aggregate to test"
        );
    }

    /// ★★★ **THE WARNING IS SILENT FOR A STANDARD-DEDUCTION FILER** (the T9 seam review's M-1).
    ///
    /// The same $900,000 2019 loan that warns an itemizer must say nothing to a filer with no
    /// Schedule A, because the warning's own closing sentence points at a question they are never
    /// asked: *"before you answer 'were you inside EVERY home-mortgage debt limit this year?'"* —
    /// and [`crate::tax::questions::QuestionId::MortgageWithinDebtLimit`] is not live without the
    /// itemize election. Telling a filer to check a figure against a limit on a deduction they are
    /// not claiming, for a question the product will never put to them, is worse than silence.
    ///
    /// **Both halves on one fixture**, because a check that never warned would pass the first alone.
    ///
    /// Mutation: read `ri.form_1098` instead of `ri.form_1098_deducted()` in
    /// `acquisition_debt_ceiling_warning` and the standard-deduction half reds.
    #[test]
    fn the_ceiling_warning_is_silent_for_a_standard_deduction_filer() {
        use crate::tax::questions::QuestionId;
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            form_1098: vec![m1098(dec!(900000), Some(date!(2019 - 06 - 01)))],
            ..Default::default()
        };
        assert!(
            ri.schedule_a.is_none(),
            "the premise: this filer takes the standard deduction"
        );
        assert!(
            !crate::tax::questions::question_is_live(QuestionId::MortgageWithinDebtLimit, &ri),
            "…so the question the warning points at is not even asked"
        );
        assert_eq!(
            acquisition_debt_ceiling_warning(&ri, Some(ceilings())),
            None,
            "★ THE KILL: a limit on a deduction they are not claiming warns them about nothing"
        );
        // The same paper, the same loan, on a return that DOES claim the deduction.
        ri.schedule_a = Some(crate::tax::return_inputs::ScheduleAInputs::default());
        assert!(
            acquisition_debt_ceiling_warning(&ri, Some(ceilings())).is_some(),
            "…and the itemizer, holding the identical Form 1098, is warned"
        );
    }

    /// ★ N-1 (T9 seam review) — the two figures go through the module's own [`money`], like every
    /// other warning in this file. The filer read `$900000` before; this is the one warning whose
    /// entire content is a comparison of two large numbers.
    ///
    /// ★★ FR-118 — and they are THOUSANDS-SEPARATED, which is the whole reason the comparison is
    /// readable at a glance. Both halves discriminate: the positive assertion reds if the figures stop
    /// going through [`money`] (interpolate `${total}` / `${limit}` raw again → `900000`), and the
    /// negative one reds if `money` reverts to `format!("${v:.2}")` (→ `$900000.00`), which is the
    /// exact defect FR-118 closed. A grouped figure printed by some *other* path cannot satisfy both.
    #[test]
    fn the_ceiling_warning_formats_both_figures_as_money() {
        let w = warned(
            vec![m1098(dec!(900000), Some(date!(2019 - 06 - 01)))],
            FilingStatus::Single,
        )
        .expect("the premise: $900,000 of post-2017 debt is over the $750,000 ceiling");
        assert!(
            w.contains("$900,000") && w.contains("$750,000"),
            "both figures print through `money()`, thousands-separated: {w}"
        );
        assert!(
            !w.contains("$900000") && !w.contains("$750000"),
            "…and NEITHER prints ungrouped (FR-118): {w}"
        );
    }

    fn w2(f: impl FnOnce(&mut W2)) -> W2 {
        let mut w = W2 {
            employer: "ACME".into(),
            box1_wages: dec!(100000),
            box3_ss_wages: dec!(100000),
            box4_ss_withheld: dec!(6200),
            box5_medicare_wages: dec!(100000),
            box6_medicare_withheld: dec!(1450),
            ..Default::default()
        };
        f(&mut w);
        w
    }

    /// ★★★ **SEAM REVIEW M-2's KILL — THE ONE LAWFUL W-2 THAT USED TO WARN.**
    ///
    /// An employee whose wages could not cover the Social Security / Medicare tax due on their
    /// reported TIPS receives a W-2 with box 4 and/or box 6 short by exactly that shortfall, and
    /// the shortfall is recorded in box 12 as code A or code B — *"Do not include this amount in
    /// box 4"* / *"… in box 6"* (`iw2w3--2026.txt:2412-2423`). Both checks fired on it. It is the
    /// one genuine false positive R4's "every one of these has an edge case" anticipated, and it is
    /// named on the form itself.
    ///
    /// ★ The kill has BOTH halves: the code silences the check, and the SAME W-2 without the code
    ///   still warns — otherwise the suppression could be a check that stopped checking.
    #[test]
    fn uncollected_tax_on_tips_silences_its_own_box_and_nothing_else() {
        use crate::tax::return_inputs::Box12Entry;
        let code = |c: &str| Box12Entry {
            code: c.to_string(),
            amount: dec!(310),
        };
        let short_box4 = |w: &mut W2| w.box4_ss_withheld = dec!(5890); // 6.2% would be 6,200
        let short_box6 = |w: &mut W2| w.box6_medicare_withheld = dec!(1140); // 1.45% ⇒ 1,450

        // (1) THE PREMISE — without the code, each shortfall warns. If this stops holding, the
        //     suppression below is asserted over nothing.
        for (what, mutate) in [
            ("box 4", &short_box4 as &dyn Fn(&mut W2)),
            ("box 6", &short_box6),
        ] {
            let ri = ReturnInputs {
                w2s: vec![w2(|w| mutate(w))],
                ..Default::default()
            };
            assert_eq!(
                transcription_warnings(&ri, Some(dec!(168600)), None).len(),
                1,
                "the premise: a short {what} with NO box-12 code warns"
            );
        }

        // (2) THE KILL — code A silences box 4, code B silences box 6.
        for (what, mutate, c) in [
            ("box 4", &short_box4 as &dyn Fn(&mut W2), "A"),
            ("box 6", &short_box6, "B"),
        ] {
            let ri = ReturnInputs {
                w2s: vec![w2(|w| {
                    mutate(w);
                    w.box12 = vec![code(c)];
                })],
                ..Default::default()
            };
            assert_eq!(
                transcription_warnings(&ri, Some(dec!(168600)), None),
                Vec::new(),
                "★ THE KILL: box 12 code {c} says the employer could not collect the tax on tips \
                 and must NOT include it in {what} — warning there trains the filer to ignore the \
                 class"
            );
        }

        // (3) PER BOX, not per W-2: code A does not excuse a wrong box 6.
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                short_box6(w);
                w.box12 = vec![code("A")];
            })],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, Some(dec!(168600)), None).len(),
            1,
            "★ THE KILL: code A is about box 4 alone — a blanket \"has A or B\" suppression would \
             lose the box-6 check on this W-2"
        );
    }

    /// A correctly transcribed W-2 raises nothing — or every other assertion here is vacuous.
    #[test]
    fn a_correct_w2_raises_no_warning() {
        let ri = ReturnInputs {
            w2s: vec![w2(|_| {})],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, Some(dec!(168600)), None),
            Vec::new(),
            "a W-2 whose boxes agree must be silent"
        );
    }

    /// ★★★ **THE PLANTED SLIP R4 NAMES: box 1 typed into box 3.** It fires all three W-2 checks at
    ///     once on a real household — the wages are over the base, so box 3 exceeds it, and box 4
    ///     (correctly 6.2% of the REAL Social Security wages) no longer matches the inflated box 3.
    ///
    /// ★ And the kill that matters more than the firing: **the row is byte-identical afterwards.**
    ///   A "warning" that quietly corrected a box would be writing testimony the filer never gave.
    #[test]
    fn a_box_1_in_box_3_slip_warns_and_writes_nothing() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box1_wages = dec!(250000);
                w.box3_ss_wages = dec!(250000); // ← the slip: the wage base caps it at 168,600
                w.box4_ss_withheld = dec!(10453.20); // 6.2% of the REAL 168,600
                w.box5_medicare_wages = dec!(250000);
                w.box6_medicare_withheld = dec!(4075); // 1.45% + the 0.9% surtax over 200,000
            })],
            ..Default::default()
        };
        let before = ri.clone();
        let got = transcription_warnings(&ri, Some(dec!(168600)), None);
        assert_eq!(
            got.len(),
            2,
            "the slip breaks the box-4 rate AND the wage base: {got:#?}"
        );
        assert!(
            got.iter().any(|w| w.message.contains("wage base")),
            "the wage-base ceiling must be named: {got:#?}"
        );
        assert!(
            got.iter().any(|w| w.message.contains("6.2% of box 3")),
            "the box-4 rate must be named: {got:#?}"
        );
        assert!(
            got.iter()
                .all(|w| w.document == WarnedDocument::W2 && w.row == 0),
            "each warning must name the ROW the filer has to open: {got:#?}"
        );
        // ★★★ THE KILL: nothing was written.
        assert_eq!(ri, before, "a warning must never change a stored value");
    }

    /// The 0.9% Additional Medicare Tax lives in box 6 too, so a high earner is NOT warned — the
    /// check brackets [1.45%, 2.35%] rather than pinning one rate. Without this the warning would
    /// fire on every filer over $200,000 and be trained away.
    #[test]
    fn the_additional_medicare_tax_in_box_6_is_not_a_slip() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box5_medicare_wages = dec!(300000);
                // 1.45% × 300,000 + 0.9% × 100,000 = 4,350 + 900
                w.box6_medicare_withheld = dec!(5250);
                w.box3_ss_wages = dec!(168600);
                w.box4_ss_withheld = dec!(10453.20);
            })],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, Some(dec!(168600)), None),
            Vec::new()
        );
        // …and a box 6 outside the bracket still warns, so the widening did not disarm the check.
        let mut broken = ri.clone();
        broken.w2s[0].box6_medicare_withheld = dec!(30000);
        let got = transcription_warnings(&broken, Some(dec!(168600)), None);
        assert_eq!(got.len(), 1, "{got:#?}");
        assert!(got[0].message.contains("Additional Medicare Tax"));
    }

    /// ★ The wage-base check is PARAMS-GATED: on a year whose table has not arrived it does not run,
    ///   rather than guessing a base. The other two still do.
    #[test]
    fn the_wage_base_check_waits_for_the_years_table() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box3_ss_wages = dec!(9999999);
                w.box4_ss_withheld = dec!(619999.94); // 6.2%, so only the base check can fire
            })],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, None, None),
            Vec::new(),
            "with no table there is no base to compare against"
        );
        assert_eq!(
            transcription_warnings(&ri, Some(dec!(168600)), None).len(),
            1
        );
    }

    /// ★★★ **THE ALL-ZERO ROW, on every document whose ISSUER has a threshold — and not on the W-2,
    ///     whose employer must issue one whatever the wages.**
    #[test]
    fn an_all_zero_row_warns_on_the_information_returns_and_never_on_a_w2() {
        let ri = ReturnInputs {
            w2s: vec![W2 {
                employer: "ACME".into(),
                ..Default::default()
            }],
            int_1099: vec![Form1099Int {
                payer: "First Bank".into(),
                box4_fed_withheld: dec!(5), // withholding is NOT income: the row is still empty
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                payer: "Fund".into(),
                ..Default::default()
            }],
            g_1099: vec![Form1099G {
                payer: "State".into(),
                ..Default::default()
            }],
            form_1098e: vec![Form1098E {
                lender: "Servicer".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let before = ri.clone();
        let got = transcription_warnings(&ri, Some(dec!(168600)), None);
        let docs: Vec<WarnedDocument> = got.iter().map(|w| w.document).collect();
        assert_eq!(
            docs,
            vec![
                WarnedDocument::Form1099Int,
                WarnedDocument::Form1099Div,
                WarnedDocument::Form1099G,
                WarnedDocument::Form1098E
            ],
            "an all-zero W-2 is ordinary (an employer issues one at any wage) and must NOT warn; \
             each information return must: {got:#?}"
        );
        assert!(
            got[0].message.contains("$10 or more of interest"),
            "the message names the ISSUER'S own threshold: {}",
            got[0].message
        );
        // ★★★ THE KILL: nothing was written.
        assert_eq!(ri, before, "a warning must never change a stored value");
    }

    /// A row with an income box filled is silent — otherwise the all-zero check is just "warn on
    /// every row" and the fixture above proves nothing.
    #[test]
    fn a_populated_information_return_row_is_silent() {
        let ri = ReturnInputs {
            int_1099: vec![Form1099Int {
                payer: "First Bank".into(),
                box10_market_discount: dec!(12), // ★ box 10 alone counts as income
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(transcription_warnings(&ri, None, None), Vec::new());
    }
}
