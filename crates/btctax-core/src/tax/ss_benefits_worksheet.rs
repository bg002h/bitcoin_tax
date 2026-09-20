//! **The Social Security Benefits Worksheet — Lines 6a and 6b, all 18 lines, in the form's own numbering.**
//!
//! ★★★ **This is the worksheet this spec BUILDS.** Its sibling, the Simplified Method Worksheet, is the
//! opposite call and is REFUSED (`SPEC_retirement_income.md` §5, R-3/R-5) — a distinction that took
//! fold-review r2's I-3 to correct, because two folds had claimed the Simplified Method was the one being
//! built and used it as authority for a line-5a rule.
//!
//! Transcribed from `design/forms/extract/i1040gi--2025.txt:3364-3468` (and `--2024.txt:3334-3438`), per
//! the standing rule: one field per numbered line, in the form's numbering, carrying the official
//! instruction text verbatim. Every quote below is held against BOTH archived editions by
//! `tests::every_line_is_verbatim_in_both_editions`.
//!
//! ## What differs between the two revisions — MEASURED, not assumed
//!
//! **17 of the 18 lines are byte-identical across TY2024 and TY2025.** Only two facts differ, and both
//! matter:
//!
//! | | TY2024 | TY2025 |
//! |---|---|---|
//! | line 3's operands | *"lines 1z, 2b, 3b, 4b, 5b, **7**, and 8"* | *"… **7a** …"* |
//! | the MFS-lived-apart disclosure | a **write-in "D"** beside the word *"benefits"* on line 6a | a **checkbox on line 6d** |
//!
//! ★★ **The second is review r1's C-1 (Critical), and it is why this is a per-revision transcription
//! rather than one table.** The spec had built a line-6d checkbox for both years — but TY2024's Form 1040
//! HAS NO LINE 6d, and its instruction carries a consequence the checkbox version does not: *"If you
//! don't, you may get a math error notice from the IRS."* A filer sent to a box that does not exist makes
//! no disclosure at all.
//!
//! ## The two STOPs are the shape of the whole worksheet
//!
//! Lines 7 and 9 each ask a question whose **No** branch ends the worksheet with *"None of your social
//! security benefits are taxable. Enter -0- on Form 1040 or 1040-SR, line 6b."* That `-0-` is a PRINTED
//! ZERO and it is testimony — *"none of my benefits are taxable"* — which is why it is not the same value
//! as the blank a filer with no benefits at all files. [`Outcome`] keeps them apart.
//!
//! ## Not indexed, and that is the finding rather than an omission
//!
//! The four thresholds — $32,000 / $25,000 on line 8, $12,000 / $9,000 on line 10 — are §86(c) statutory
//! amounts with no inflation adjustment, and they are byte-identical in both editions.
//! `tests::the_four_thresholds_are_not_indexed_and_both_editions_agree` reads them out of both extracts
//! rather than pinning a literal, so an edition that DID change one reds instead of being absorbed.

use crate::conventions::Usd;
use crate::tax::FilingStatus;
use rust_decimal_macros::dec;

/// **Every line of the worksheet, with its instruction text VERBATIM.**
///
/// Line 3 is absent on purpose: its operand list is the one line that differs between revisions, so it
/// comes from [`line3_operands`] and cannot be quoted once for both years.
pub const LINES: &[(u8, &str)] = &[
    (1, "Enter the total amount from box 5 of all your Forms SSA-1099 and"),
    (2, "Multiply line 1 by 50% (0.50)"),
    // (3) — see `line3_operands`.
    (4, "Enter the amount, if any, from Form 1040 or 1040-SR, line 2a"),
    (5, "Combine lines 2, 3, and 4"),
    (
        6,
        "Enter the total of the amounts from Schedule 1, lines 11 through 20, and 23 and 25",
    ),
    (7, "Is the amount on line 6 less than the amount on line 5?"),
    (8, "• Married filing jointly, enter $32,000"),
    (9, "Is the amount on line 8 less than the amount on line 7?"),
    (
        10,
        "Enter $12,000 if married filing jointly; $9,000 if single, head of household, qualifying surviving",
    ),
    (11, "Subtract line 10 from line 9. If zero or less, enter -0-"),
    (12, "Enter the smaller of line 9 or line 10"),
    (13, "Enter one-half of line 12"),
    (14, "Enter the smaller of line 2 or line 13"),
    (
        15,
        "Multiply line 11 by 85% (0.85). If line 11 is zero, enter -0-",
    ),
    (16, "Add lines 14 and 15"),
    (17, "Multiply line 1 by 85% (0.85)"),
    (
        18,
        "Taxable social security benefits. Enter the smaller of line 16 or line 17. Also enter this amount",
    ),
];

/// ★★★ **Line 3's operand list, which is the one line that moved between revisions.**
///
/// TY2024 names Form 1040 line **7**; TY2025 names line **7a**. One character, and it is a reference to a
/// different cell — exactly the class of defect that produced the Form 6251 line-33 error this repo's
/// transcription rule was written for (a rendered `12` read where the form says `22`).
///
/// Returns `None` for an edition this module has not been shown, because guessing an operand list is how
/// a future year silently inherits the wrong one.
#[must_use]
pub fn line3_operands(edition: &str) -> Option<&'static str> {
    match edition {
        "2024" => Some(
            "Combine the amounts from Form 1040 or 1040-SR, lines 1z, 2b, 3b, 4b, 5b, 7, and 8",
        ),
        "2025" => Some(
            "Combine the amounts from Form 1040 or 1040-SR, lines 1z, 2b, 3b, 4b, 5b, 7a, and 8",
        ),
        _ => None,
    }
}

/// How a married-filing-separately filer who lived apart all year makes that disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfsLivedApartDisclosure {
    /// **TY2024 — a write-in `"D"` beside the word *"benefits"* on line 6a. There is NO line 6d.**
    ///
    /// Review r1's C-1: the spec had built a line-6d checkbox for this year, on a form that does not
    /// print one. The consequence is the instruction's own and it is not in the TY2025 wording.
    WriteInDBesideBenefitsOnLine6a,
    /// **TY2025 — a checkbox on line 6d**, which that revision added.
    CheckboxOnLine6d,
}

/// Which disclosure a given edition requires. `None` for an edition not transcribed here.
#[must_use]
pub fn mfs_lived_apart_disclosure(edition: &str) -> Option<MfsLivedApartDisclosure> {
    match edition {
        "2024" => Some(MfsLivedApartDisclosure::WriteInDBesideBenefitsOnLine6a),
        "2025" => Some(MfsLivedApartDisclosure::CheckboxOnLine6d),
        _ => None,
    }
}

/// **§86(c) line-8 base amount.** Not indexed; identical in both editions.
#[must_use]
pub fn line8_base(status: FilingStatus, mfs_lived_apart: bool) -> Usd {
    match status {
        FilingStatus::Mfj => dec!(32_000),
        // "Single, head of household, qualifying surviving spouse, or married filing separately and you
        //  lived apart from your spouse for all of <year>, enter $25,000"
        FilingStatus::Mfs if !mfs_lived_apart => Usd::ZERO, // the lived-WITH branch skips line 8 entirely
        _ => dec!(25_000),
    }
}

/// **§86(c)(2) line-10 adjusted base amount.** Not indexed; identical in both editions.
#[must_use]
pub fn line10_adjusted_base(status: FilingStatus, mfs_lived_apart: bool) -> Usd {
    match status {
        FilingStatus::Mfj => dec!(12_000),
        FilingStatus::Mfs if !mfs_lived_apart => Usd::ZERO,
        _ => dec!(9_000),
    }
}

/// What the worksheet concluded. ★★★ The two `-0-` outcomes are NOT the same as filing nothing: they are
/// the filer testifying *"none of my benefits are taxable"* on a line that prints `-0-`, which is
/// different from the blank a filer with no benefits files. Keeping them apart is what stops a printed
/// zero and an absent line collapsing into one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Line 7's **No** branch: line 6 is not less than line 5. STOP; 6b prints `-0-`.
    NoneTaxableStoppedAtLine7,
    /// Line 9's **No** branch: line 8 is not less than line 7. STOP; 6b prints `-0-`.
    NoneTaxableStoppedAtLine9,
    /// Line 18 — the smaller of line 16 and line 17.
    Taxable(Usd),
}

impl Outcome {
    /// The figure Form 1040 line 6b prints. Both STOPs print `-0-`, per the worksheet's own words.
    #[must_use]
    pub fn line_6b(self) -> Usd {
        match self {
            Outcome::NoneTaxableStoppedAtLine7 | Outcome::NoneTaxableStoppedAtLine9 => Usd::ZERO,
            Outcome::Taxable(v) => v,
        }
    }
}

/// Everything the worksheet reads that is not a Social Security figure. Each field names the 1040 or
/// Schedule 1 line it comes from, because the worksheet cites lines rather than concepts.
#[derive(Debug, Clone, Copy)]
pub struct OtherIncome {
    /// Line 3 — the combined 1040 operands (1z, 2b, 3b, 4b, 5b, 7-or-7a, 8).
    pub line3_combined: Usd,
    /// Line 4 — 1040 line 2a, tax-exempt interest.
    pub line4_tax_exempt_interest: Usd,
    /// ★★★ Line 6 — *"the total of the amounts from Schedule 1, lines 11 through 20, and 23 and 25"*.
    ///
    /// A **BLOCK**, not a list of two operands. FR-183/I-9: an earlier draft prescribed
    /// *"11 through 20"* and dropped *"and 23 and 25"*, which would understate line 6 and therefore
    /// OVERSTATE the taxable benefit. OpenTaxSolver's own comment corroborates the block reading —
    /// *"This calc. depends on line L6a and Sched1[11-25]"*.
    pub line6_schedule_1_block: Usd,
}

/// **Run the worksheet.** `line1` is Σ box 5 over every Form SSA-1099 and RRB-1099 of both spouses.
///
/// ★★ `mfs_lived_apart` is only consulted when `status` is [`FilingStatus::Mfs`], and the lived-WITH
/// branch is the one with no oracle witness at all (review r1's C-3: both engines treat every MFS filer as
/// lived-apart and AGREE while doing it, so a green two-oracle sweep proves nothing there). Its arithmetic
/// is therefore transcribed with extra care: line 8's own text says *"skip lines 8 through 15; multiply
/// line 7 by 85% (0.85) and enter the result on line 16. Then, go to line 17"*.
#[must_use]
pub fn run(line1: Usd, status: FilingStatus, mfs_lived_apart: bool, other: OtherIncome) -> Outcome {
    let line2 = line1 * dec!(0.50);
    let line5 = line2 + other.line3_combined + other.line4_tax_exempt_interest;
    let line6 = other.line6_schedule_1_block;
    // L7 — "Is the amount on line 6 less than the amount on line 5?" No ⇒ STOP.
    if line6 >= line5 {
        return Outcome::NoneTaxableStoppedAtLine7;
    }
    let line7 = line5 - line6;

    let lived_with = status == FilingStatus::Mfs && !mfs_lived_apart;
    let line17 = line1 * dec!(0.85);
    if lived_with {
        // "skip lines 8 through 15; multiply line 7 by 85% (0.85) and enter the result on line 16.
        //  Then, go to line 17" — so line 18 is min(line 16, line 17) with no threshold at all.
        let line16 = line7 * dec!(0.85);
        return Outcome::Taxable(line16.min(line17));
    }

    let line8 = line8_base(status, mfs_lived_apart);
    // L9 — "Is the amount on line 8 less than the amount on line 7?" No ⇒ STOP.
    if line8 >= line7 {
        return Outcome::NoneTaxableStoppedAtLine9;
    }
    let line9 = line7 - line8;
    let line10 = line10_adjusted_base(status, mfs_lived_apart);
    // "Subtract line 10 from line 9. If zero or less, enter -0-"
    let line11 = (line9 - line10).max(Usd::ZERO);
    let line12 = line9.min(line10);
    let line13 = line12 * dec!(0.50);
    let line14 = line2.min(line13);
    // "Multiply line 11 by 85% (0.85). If line 11 is zero, enter -0-"
    let line15 = line11 * dec!(0.85);
    let line16 = line14 + line15;
    Outcome::Taxable(line16.min(line17))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(edition: &str) -> String {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(format!("design/forms/extract/i1040gi--{edition}.txt"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    /// ★★★ **THE TRANSCRIPTION GATE — every line verbatim in BOTH archived editions.**
    ///
    /// The standing rule exists because of Form 6251 line 33: a rendered `12` transcribed where the form
    /// says `22`, which taxed one slice twice and inflated the tentative minimum tax by $200,000 on a
    /// single vector. No review caught it; running the transcription against the extract caught it in
    /// seconds. This is that check for this worksheet.
    #[test]
    fn every_line_is_verbatim_in_both_editions() {
        for ed in ["2024", "2025"] {
            let src = extract(ed);
            for (n, text) in LINES {
                assert!(
                    src.contains(text),
                    "i1040gi--{ed} does not print worksheet line {n}'s text: {text:?}"
                );
            }
            // Line 3 is per-revision by construction.
            let l3 = line3_operands(ed).expect("both editions are transcribed");
            assert!(src.contains(l3), "i1040gi--{ed} line 3: {l3:?}");
        }
    }

    /// ★★★ **Line 3 is the one line that MOVED, and each edition's operand list must appear ONLY in its
    /// own edition.** Both directions, because a transcription that quoted TY2025's list for TY2024 would
    /// point line 3 at a cell that revision does not have — the Form 6251 line-33 shape exactly.
    #[test]
    fn line_3s_operand_list_is_edition_specific_in_both_directions() {
        let (y24, y25) = (extract("2024"), extract("2025"));
        let (l24, l25) = (
            line3_operands("2024").unwrap(),
            line3_operands("2025").unwrap(),
        );
        assert_ne!(
            l24, l25,
            "the two lists must differ, or this test is decorative"
        );
        assert!(
            y24.contains(l24) && !y24.contains(l25),
            "TY2024 must name line 7, not 7a"
        );
        assert!(
            y25.contains(l25) && !y25.contains(l24),
            "TY2025 must name line 7a, not 7"
        );
        // ★ An untranscribed edition returns `None` rather than inheriting a neighbour's list.
        assert_eq!(line3_operands("2026"), None);
        assert_eq!(line3_operands(""), None);
    }

    /// ★★★ **C-1 (review r1, CRITICAL) — the MFS-lived-apart disclosure, per revision.**
    ///
    /// TY2024 has **no line 6d at all**: the disclosure is a write-in `"D"` beside the word *"benefits"*
    /// on line 6a, and its instruction carries a consequence the TY2025 wording does not. The spec had
    /// built a 6d checkbox for both years; a filer sent to a box that does not exist makes no disclosure.
    #[test]
    fn c1_the_mfs_disclosure_differs_by_revision_and_ty2024_has_no_line_6d() {
        assert_eq!(
            mfs_lived_apart_disclosure("2024"),
            Some(MfsLivedApartDisclosure::WriteInDBesideBenefitsOnLine6a)
        );
        assert_eq!(
            mfs_lived_apart_disclosure("2025"),
            Some(MfsLivedApartDisclosure::CheckboxOnLine6d)
        );
        assert_eq!(mfs_lived_apart_disclosure("2026"), None);

        let (y24, y25) = (extract("2024"), extract("2025"));
        // The TY2024 instruction, verbatim, including the consequence.
        assert!(
            y24.contains("enter “D” to"),
            "TY2024's write-in instruction"
        );
        assert!(
            y24.contains("you may get a math error notice from the IRS"),
            "TY2024 names the consequence of omitting the disclosure, and TY2025 does not"
        );
        assert!(
            !y24.contains("check the box on line 6d"),
            "TY2024 has no line 6d"
        );
        assert!(y25.contains("check the box on line 6d"), "TY2025 does");
        // ★★ And the FORM itself agrees — this is the check that makes it a fact about the form rather
        //    than about the instructions' wording.
        let form = |ed: &str| {
            std::fs::read_to_string(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(format!("design/forms/extract/f1040--{ed}.txt")),
            )
            .unwrap()
        };
        assert!(
            !form("2024").contains("6d"),
            "TY2024 Form 1040 prints no 6d"
        );
        assert!(form("2025").contains("6d"), "TY2025 Form 1040 prints 6d");
    }

    /// ★★★ **M-10 — the four §86(c) thresholds are NOT indexed, read out of BOTH extracts.**
    ///
    /// Pinning literals here would assert what this module already says. Reading them from the archived
    /// instructions and requiring both editions to agree is what makes "not indexed" a measurement: an
    /// edition that DID change one reds instead of being absorbed.
    #[test]
    fn the_four_thresholds_are_not_indexed_and_both_editions_agree() {
        for ed in ["2024", "2025"] {
            let src = extract(ed);
            for amount in ["$32,000", "$25,000", "$12,000", "$9,000"] {
                assert!(
                    src.contains(amount),
                    "i1040gi--{ed} does not print the threshold {amount}"
                );
            }
        }
        // And the constants this module computes with are those four.
        assert_eq!(line8_base(FilingStatus::Mfj, false), dec!(32_000));
        assert_eq!(line8_base(FilingStatus::Single, false), dec!(25_000));
        assert_eq!(line8_base(FilingStatus::Mfs, true), dec!(25_000));
        assert_eq!(line10_adjusted_base(FilingStatus::Mfj, false), dec!(12_000));
        assert_eq!(
            line10_adjusted_base(FilingStatus::Single, false),
            dec!(9_000)
        );
        assert_eq!(line10_adjusted_base(FilingStatus::Mfs, true), dec!(9_000));
    }

    fn other(line3: i64, tax_exempt: i64, sch1: i64) -> OtherIncome {
        OtherIncome {
            line3_combined: rust_decimal::Decimal::from(line3),
            line4_tax_exempt_interest: rust_decimal::Decimal::from(tax_exempt),
            line6_schedule_1_block: rust_decimal::Decimal::from(sch1),
        }
    }

    /// ★★ **Both STOPs, and they are DISTINCT outcomes rather than one zero.**
    ///
    /// Line 7's No branch and line 9's No branch both print `-0-` on 6b, but they are different findings
    /// about the filer and the worksheet says so in two different places. A single `Usd::ZERO` return
    /// would make them indistinguishable, and `-0-` on 6b is testimony — *"none of my benefits are
    /// taxable"* — not the blank a filer with no benefits files.
    #[test]
    fn both_stops_are_distinct_and_each_prints_a_zero_on_6b() {
        // L7 No: line 6 (Schedule 1 block) is not less than line 5.
        let o = run(
            dec!(20_000),
            FilingStatus::Single,
            false,
            other(10_000, 0, 99_000),
        );
        assert_eq!(o, Outcome::NoneTaxableStoppedAtLine7);
        assert_eq!(o.line_6b(), Usd::ZERO);

        // L9 No: line 8 ($25,000) is not less than line 7.
        let o = run(
            dec!(10_000),
            FilingStatus::Single,
            false,
            other(5_000, 0, 0),
        );
        assert_eq!(
            o,
            Outcome::NoneTaxableStoppedAtLine9,
            "line 7 = 10,000 < 25,000"
        );
        assert_eq!(o.line_6b(), Usd::ZERO);

        assert_ne!(
            Outcome::NoneTaxableStoppedAtLine7,
            Outcome::NoneTaxableStoppedAtLine9,
            "the two STOPs must not collapse into one value"
        );
    }

    /// ★★★ **The 85% ceiling — line 18 can never exceed 85% of line 1, whatever the other income.**
    ///
    /// §86(a)(2)'s cap, and the worksheet enforces it structurally as `min(line 16, line 17)`. This is the
    /// invariant worth asserting over a range rather than at a point: a transcription that dropped line 17
    /// from the final `min` would pass any single worked example whose line 16 happened to be smaller.
    #[test]
    fn line_18_never_exceeds_85_percent_of_line_1() {
        for benefits in [1_000, 12_000, 20_000, 40_000] {
            for extra in [0, 25_000, 60_000, 250_000, 1_000_000] {
                for status in [FilingStatus::Single, FilingStatus::Mfj] {
                    let o = run(
                        rust_decimal::Decimal::from(benefits),
                        status,
                        false,
                        other(extra, 0, 0),
                    );
                    let cap = rust_decimal::Decimal::from(benefits) * dec!(0.85);
                    assert!(
                        o.line_6b() <= cap,
                        "{benefits}/{extra}/{status:?}: 6b {} exceeds the 85% cap {cap}",
                        o.line_6b()
                    );
                }
            }
        }
    }

    /// ★★★ **The MFS-lived-WITH branch — the one with NO oracle witness (review r1's C-3).**
    ///
    /// Both engines treat every MFS filer as lived-apart and AGREE while doing it, so a green two-oracle
    /// sweep proves nothing here. The worksheet's own words are therefore the only authority: *"skip lines
    /// 8 through 15; multiply line 7 by 85% (0.85) and enter the result on line 16. Then, go to line 17."*
    ///
    /// ★★ Consequences the arithmetic must show, and each is a way the branch could be got wrong:
    /// there is NO threshold at all (so a small benefit is taxable where a single filer's would not be),
    /// line 2's 50% half never enters, and the 85% ceiling still binds.
    #[test]
    fn c3_the_mfs_lived_with_branch_has_no_threshold_and_still_caps_at_85_percent() {
        let benefits = dec!(10_000);
        // A lived-WITH MFS filer with only $5,000 of other income: line 7 = 5,000 + 5,000 = 10,000,
        // line 16 = 8,500, line 17 = 8,500 ⇒ 8,500 taxable.
        let with = run(benefits, FilingStatus::Mfs, false, other(5_000, 0, 0));
        assert_eq!(with, Outcome::Taxable(dec!(8_500)));

        // The SAME figures as lived-apart stop at line 9, because $25,000 is not less than line 7.
        let apart = run(benefits, FilingStatus::Mfs, true, other(5_000, 0, 0));
        assert_eq!(
            apart,
            Outcome::NoneTaxableStoppedAtLine9,
            "the threshold applies on the lived-APART branch and not on the lived-with one — that \
             difference is the whole of C-3"
        );

        // ★ The ceiling still binds on the lived-with branch: huge other income cannot push 6b past 85%.
        let huge = run(benefits, FilingStatus::Mfs, false, other(500_000, 0, 0));
        assert_eq!(huge.line_6b(), benefits * dec!(0.85));
    }

    /// ★★★ **THE TWO-ORACLE KAT — six cases reconciled against Tax-Calculator 6.8.2, computed live on
    /// 2026-09-20, plus OpenTaxSolver's implementation read line-for-line.**
    ///
    /// The standing rule is two engines, never one. This worksheet is unusual in that BOTH oracles
    /// genuinely compute it — my earlier claim that §G-9 applied here (that they take line 6b as an
    /// INPUT) was FALSE and review r1's I-1 corrected it. So they are real witnesses.
    ///
    /// ★★ **taxcalc's figures**, from `Calculator.dataframe(['c02500'])` at FLPDYR 2024 with `e02400` as
    /// gross benefits and `e00200` as wages:
    ///
    /// | benefits | wages | status | taxcalc `c02500` |
    /// |---|---|---|---|
    /// | 20,000 | 10,000 | Single | 0 |
    /// | 10,000 | 5,000 | Single | 0 |
    /// | 20,000 | 40,000 | MFJ | 11,100 |
    /// | 20,000 | 30,000 | MFJ | 4,000 |
    /// | 10,000 | 5,000 | MFS | 0 |
    /// | 40,000 | 250,000 | Single | 34,000 |
    ///
    /// ★★★ **And OTS agrees STRUCTURALLY, step for step**, which is the stronger form of agreement
    /// because it is not sensitive to the cases anyone happened to pick. From
    /// `taxsolve_US_1040_2024.c`'s `SocSec_Worksheet`: `ws[2] = 0.5 * ws[1]`,
    /// `ws[5] = ws[2] + ws[3] + ws[4]`, `ws[6] = Σ Sched1[11..20] + Sched1[23] + Sched1[25]` — **the
    /// BLOCK, exactly as FR-183/I-9 says and not the "11 through 20" an earlier draft prescribed** —
    /// `ws[11] = NotLessThanZero(ws[9] - ws[10])`, `ws[12] = smallerof(ws[9], ws[10])`,
    /// `ws[14] = smallerof(ws[2], ws[13])`, `ws[18] = smallerof(ws[16], ws[17])`. Every step matches.
    ///
    /// ★ One difference, recorded rather than reconciled: OTS applies `Conditional_Round` to ws[13],
    /// ws[15] and ws[17]. The worksheet's own text instructs no rounding at those steps, so this module
    /// does not round there and btctax's own rounding convention applies at the printed line instead.
    #[test]
    fn the_two_oracles_reconcile_on_six_cases() {
        let cases: [(i64, i64, FilingStatus, i64); 9] = [
            (20_000, 10_000, FilingStatus::Single, 0),
            (10_000, 5_000, FilingStatus::Single, 0),
            (20_000, 40_000, FilingStatus::Mfj, 11_100),
            (20_000, 30_000, FilingStatus::Mfj, 4_000),
            // ★ MFS here is the LIVED-APART branch, which is the only one either oracle models.
            (10_000, 5_000, FilingStatus::Mfs, 0),
            (40_000, 250_000, FilingStatus::Single, 34_000),
            // ★★★ The case that makes line 14's `smaller of line 2 or line 13` OBSERVABLE, found by
            //     mutation: line 9 (8,000) is BELOW line 10 (12,000) so line 11 and therefore line 15 are
            //     zero, and line 1 (4,000) is below line 9 — so line 16 IS line 14 and the 85% ceiling
            //     does not mask it. Dropping the `min` files 3,400 instead of 2,000. taxcalc: 2,000.
            (4_000, 38_000, FilingStatus::Mfj, 2_000),
            (10_000, 0, FilingStatus::Single, 0),
            (20_000, 10_000, FilingStatus::Mfj, 0),
        ];
        for (benefits, wages, status, expected) in cases {
            let lived_apart = status == FilingStatus::Mfs;
            let got = run(
                rust_decimal::Decimal::from(benefits),
                status,
                lived_apart,
                other(wages, 0, 0),
            );
            assert_eq!(
                got.line_6b(),
                rust_decimal::Decimal::from(expected),
                "benefits {benefits}, wages {wages}, {status:?}: taxcalc says {expected}, this \
                 worksheet says {}",
                got.line_6b()
            );
        }
    }

    /// ★★★ **C-3, PROVEN FROM BOTH ENGINES' SOURCE rather than inferred from a sweep: the MFS
    /// lived-WITH branch has ZERO witnesses, and both oracles are wrong in the same direction.**
    ///
    /// * **OTS**: `if (status == MARRIED_FILING_JOINTLY) ws[8] = 32000.0; else ws[8] = 25000.0;` — an MFS
    ///   filer gets the $25,000 threshold, and the file contains **no** lived-with test at all.
    /// * **taxcalc**: `SSBenefits` is 29 lines and contains **no** `MARS == 3` special case.
    ///
    /// Both therefore give a lived-WITH MFS filer a threshold the worksheet says they do not get, which
    /// UNDERSTATES the taxable benefit — and because they agree, a green two-oracle sweep on such a
    /// household proves nothing. This test is the only thing standing behind that branch, so it asserts
    /// the arithmetic AND the size of the disagreement.
    #[test]
    fn c3_neither_oracle_models_the_lived_with_branch_and_both_understate_it() {
        let (benefits, wages) = (dec!(10_000), 5_000);
        let correct = run(benefits, FilingStatus::Mfs, false, other(wages, 0, 0));
        // The worksheet: skip lines 8-15, line 16 = 85% of line 7, line 18 = min(16, 17).
        assert_eq!(correct, Outcome::Taxable(dec!(8_500)));

        // What BOTH oracles compute instead — the lived-apart path, which stops at line 9.
        let as_the_oracles_see_it = run(benefits, FilingStatus::Mfs, true, other(wages, 0, 0));
        assert_eq!(as_the_oracles_see_it, Outcome::NoneTaxableStoppedAtLine9);

        let understatement = correct.line_6b() - as_the_oracles_see_it.line_6b();
        assert_eq!(
            understatement,
            dec!(8_500),
            "the whole taxable benefit is the size of the oracles' blind spot on this household — so a \
             reconciliation against either engine here would confirm a figure $8,500 too low"
        );
    }

    /// ★★★ **Three cases that exist because MUTATION found the tests could not see these steps.**
    ///
    /// Each of the three was a surviving mutant against the first version of this module's tests — a
    /// change to the worksheet's arithmetic that every one of eleven tests accepted. They are kept
    /// together because they share a cause: **the 85% ceiling on line 17 masks an error in line 16
    /// whenever line 16 is the larger of the two**, so a case can only see a line-16 step if it is
    /// constructed to make line 16 bind.
    #[test]
    fn the_three_steps_mutation_showed_were_invisible() {
        // ── (1) Line 7's comparison is `line 6 >= line 5`, not `>`. ──────────────────────────────────
        //    Only an EXACT equality distinguishes them, and the two branches differ in WHICH STOP is
        //    reached — which matters, because the two STOPs are different findings about the filer.
        let equal = run(
            dec!(20_000),
            FilingStatus::Single,
            false,
            other(30_000, 0, 40_000),
        );
        assert_eq!(
            equal,
            Outcome::NoneTaxableStoppedAtLine7,
            "line 5 = 10,000 + 30,000 = 40,000 and line 6 = 40,000; the worksheet asks whether line 6 \
             is LESS than line 5, and it is not — so the No branch is taken at line 7, not line 9"
        );

        // ── (2) The MFS lived-WITH branch multiplies line 7 by 85%, and the CEILING is a different
        //        number. ─────────────────────────────────────────────────────────────────────────────
        //    With no other income line 7 is half the benefits, so 85% of line 7 is well under 85% of
        //    line 1 and line 16 binds. The earlier lived-with case had line 16 == line 17, so dropping
        //    the 0.85 on line 16 changed nothing.
        let lived_with = run(dec!(10_000), FilingStatus::Mfs, false, other(0, 0, 0));
        assert_eq!(
            lived_with,
            Outcome::Taxable(dec!(4_250)),
            "line 7 = 5,000; line 16 = 85% of that = 4,250; line 17 = 8,500 — so line 16 binds and the \
             0.85 on line 16 is visible"
        );

        // ── (3) Line 14 is `the smaller of line 2 or line 13`. ──────────────────────────────────────
        //    Needs line 11 = 0 (so line 15 contributes nothing), line 1 below line 9 (so line 2 is the
        //    smaller), and line 16 under the ceiling. taxcalc reconciles this case at 2,000.
        let smaller_of = run(dec!(4_000), FilingStatus::Mfj, false, other(38_000, 0, 0));
        assert_eq!(
            smaller_of,
            Outcome::Taxable(dec!(2_000)),
            "line 2 = 2,000 and line 13 = 4,000, so line 14 is 2,000; taking line 13 instead files \
             3,400 — and both oracles say 2,000"
        );
    }

    /// ★★ **Line 6 is a BLOCK, and a bigger line 6 must LOWER the taxable benefit.** FR-183/I-9: an
    /// earlier draft prescribed *"11 through 20"* and dropped *"and 23 and 25"*, which understates line 6
    /// and therefore OVERSTATES the taxable benefit. The direction is what this asserts.
    #[test]
    fn a_larger_schedule_1_block_lowers_the_taxable_benefit() {
        let base = run(dec!(20_000), FilingStatus::Mfj, false, other(40_000, 0, 0));
        let more = run(
            dec!(20_000),
            FilingStatus::Mfj,
            false,
            other(40_000, 0, 5_000),
        );
        assert!(
            more.line_6b() < base.line_6b(),
            "a larger line 6 must reduce line 6b: {:?} vs {:?}",
            more.line_6b(),
            base.line_6b()
        );
    }

    /// ★ Tax-exempt interest (line 4) RAISES the taxable benefit — the counter-intuitive one, and the
    /// reason §86 counts it: a filer who moves income into municipal bonds does not thereby shelter
    /// their benefits.
    #[test]
    fn tax_exempt_interest_raises_the_taxable_benefit() {
        let without = run(dec!(20_000), FilingStatus::Mfj, false, other(30_000, 0, 0));
        let with = run(
            dec!(20_000),
            FilingStatus::Mfj,
            false,
            other(30_000, 8_000, 0),
        );
        assert!(with.line_6b() > without.line_6b());
    }
}
