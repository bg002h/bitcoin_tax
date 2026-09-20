//! **Form SSA-1099 and Form RRB-1099 — the Social Security input surface for 1040 lines 6a and 6b.**
//!
//! ★★★ **ONE struct, TWO documents, and the box numbers do NOT line up. That is the whole reason this
//! module exists rather than a pair of `boxN_` fields.**
//!
//! `CLAUDE.md`'s rule is that a field is named for its line *within a transcription struct* — a struct
//! that says what **one revision of one document** prints. This is not that: Form SSA-1099 and Form
//! RRB-1099 report the same quantity on differently-numbered boxes, and a field called `box6_withheld`
//! would be **wrong on one of the two forms**:
//!
//! | quantity | Form SSA-1099 | Form RRB-1099 |
//! |---|---|---|
//! | benefits paid | box 3 | box 3 |
//! | benefits repaid | box 4 | box 4 |
//! | net benefits (paid − repaid) | box 5 | box 5 |
//! | **federal income tax withheld** | **box 6** | **box 10** |
//! | *what box 6 means instead* | — | **Workers' Compensation Offset** |
//!
//! Read off the archived facsimiles in `legal/text/irs-publications/Pub915_…--2025.txt`. ★★ And the
//! hazard has a third layer: **box 10 is *Federal Income Tax Withheld* on RRB-1099 and *Gross Benefit
//! Paid* on RRB-1099-R**, a different form in the same family. So the box number is derived from
//! [`SsaFormKind`] and never typed into a field name.
//!
//! ## Box 5 is DERIVED, never collected
//!
//! The facsimile prints its own definition: *"Box 5. Net Benefits for 2025 (Box 3 minus Box 4)"*.
//! Collecting all three would permit a triple that contradicts itself, and the filer cannot type the
//! negative case anyway — *"If parentheses are around the figure in box 5, it means that the figure in
//! box 4 is larger than the figure in box 3."* Applying the form's own arithmetic is transcription, not
//! a forbidden closed form (fold-review r2, I-11).
//!
//! ## R-6 sums across BOTH SPOUSES, and Pub 915 says so twice
//!
//! *"If you receive more than one form, a negative figure in box 5 of one form is used to offset a
//! positive figure in box 5 of another form for that same year"* — and the joint-return rule with its own
//! worked example: Ryan's box 5 is $3,000, Jordan's is ($500), *"Ryan and Jordan will use $2,500"*. A
//! per-form or per-spouse test would refuse a return the instructions finish in one sentence, which is a
//! refusal too WIDE — and a refusal that is too wide costs the filer their return.

use crate::conventions::Usd;
use crate::tax::return_inputs::Owner;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use time::Date;

/// Which of the two forms this row is. It decides the withholding box number and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsaFormKind {
    /// **Form SSA-1099** — Social Security benefits. Withholding is **box 6**, *"Voluntary Federal
    /// Income Tax Withheld"*.
    Ssa1099,
    /// **Form RRB-1099** — the social-security-equivalent benefit portion of tier 1 railroad
    /// retirement. Withholding is **box 10**; its box 6 is *"Workers' Compensation Offset"*, which is
    /// not withholding and which this build does not read.
    Rrb1099,
}

impl SsaFormKind {
    /// The box the FEDERAL WITHHOLDING is printed in on this form. ★ Derived rather than named in a
    /// field, because the two forms disagree and `box6_` would be a lie on one of them.
    #[must_use]
    pub fn withholding_box(self) -> &'static str {
        match self {
            SsaFormKind::Ssa1099 => "6",
            SsaFormKind::Rrb1099 => "10",
        }
    }

    /// The form's own name, for a refusal or an advisory that must say which paper it means.
    #[must_use]
    pub fn designation(self) -> &'static str {
        match self {
            SsaFormKind::Ssa1099 => "Form SSA-1099",
            SsaFormKind::Rrb1099 => "Form RRB-1099",
        }
    }
}

/// One transcribed Form SSA-1099 or RRB-1099.
///
/// ★ `owner` matters because R-6 and worksheet line 1 both sum across **both spouses' forms**, not one
/// filer's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormSsa1099 {
    pub owner: Owner,
    pub kind: SsaFormKind,
    /// **R10.2 — when this row was transcribed.**
    #[serde(default)]
    pub transcribed_on: Option<Date>,
    /// **Box 3 — benefits PAID in the year** (SSA-1099 *"Benefits Paid in 2025"*; RRB-1099 *"Gross
    /// Social Security Equivalent Benefit Portion of Tier 1 Paid"*). No `serde(default)`: a form
    /// without box 3 is a mistyped row.
    pub box3_benefits_paid: Usd,
    /// **Box 4 — benefits REPAID to the agency in the year.** Usually blank.
    #[serde(default)]
    pub box4_benefits_repaid: Usd,
    /// **Federal income tax withheld — SSA-1099 box 6, RRB-1099 box 10.** See
    /// [`SsaFormKind::withholding_box`] for why this is not named for a box.
    #[serde(default)]
    pub federal_withholding: Usd,
}

/// **Box 5 — net benefits, DERIVED.** *"Box 5. Net Benefits for 2025 (Box 3 minus Box 4)"*.
///
/// Signed: NEGATIVE when box 4 exceeds box 3, which the form prints in parentheses. `Usd` is
/// `rust_decimal::Decimal` and holds that — fold-review r2's I-11 corrected the claim that it could not.
#[must_use]
pub fn box5_net_benefits(f: &FormSsa1099) -> Usd {
    f.box3_benefits_paid - f.box4_benefits_repaid
}

/// **Worksheet line 1 — Σ box 5 over every form of BOTH spouses**, which is what the worksheet says:
/// *"Enter the total amount from box 5 of all your Forms SSA-1099 and RRB-1099."*
///
/// ★ A negative box 5 on one form OFFSETS a positive one on another, per Pub 915, so this is a plain
/// signed sum and not a sum of clamped values.
#[must_use]
pub fn worksheet_line1(rows: &[FormSsa1099]) -> Usd {
    rows.iter().map(box5_net_benefits).sum()
}

/// **Σ federal withholding over every form** → 1040 line 25b.
#[must_use]
pub fn withholding(rows: &[FormSsa1099]) -> Usd {
    rows.iter().map(|f| f.federal_withholding).sum()
}

/// ★★★ **§1341's threshold. A repayment excess ABOVE this may be deducted or credited**, and the
/// refusal names both routes because a filer told only about the deduction may take the worse of the two.
pub const SECTION_1341_THRESHOLD: Usd = dec!(3_000);

/// Why the Social Security block stops the return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsRefusal {
    /// **R-6** — Σ box 4 > Σ box 3 across every form of both spouses. *"None of your benefits are
    /// taxable … Don't use Worksheet 1 in this case."* The excess may reach Schedule A line 16 or a
    /// Schedule 3 line 13z credit marked *"I.R.C. 1341"*, neither of which this build computes.
    RepaymentsExceedBenefits {
        /// Σ box 4 − Σ box 3. Whether it exceeds [`SECTION_1341_THRESHOLD`] decides whether the §1341
        /// routes are even available, so the refusal carries it rather than making the caller redo it.
        excess: Usd,
    },
    /// **R-7** — the worksheet is BARRED for this filer by the *Exception* in the line 6a/6b
    /// instructions: *"You file Form 2555, 4563, or 8815, or you exclude employer-provided adoption
    /// benefits or income from sources within Puerto Rico. Instead, use the worksheet in Pub. 915."*
    WorksheetBarredByExclusion,
    /// ★★★ **R-8 — the MFS lived-apart declaration is unanswered, and it changes the ARITHMETIC.**
    ///
    /// Lived apart ⇒ the $25,000 / $9,000 thresholds apply. Lived with ⇒ *"skip lines 8 through 15;
    /// multiply line 7 by 85% (0.85)"*, with no threshold at all. So the same figures produce either a
    /// wholly non-taxable benefit or 85% of it taxable, and neither answer may be defaulted.
    MfsLivedApartUnanswered,
    /// **R-7's unanswered limb** — the exclusion declaration is `None`. Silence is not testimony that no
    /// exclusion applies, and guessing `false` would run a worksheet the instructions forbid.
    ExclusionUnanswered,
}

/// **The Social Security gate.** `has_exclusion` is the filer's declaration for the *Exception*'s
/// Form 2555/4563/8815 and adoption/Puerto-Rico limb.
///
/// ★★ R-6 is tested BEFORE R-7, and that order is the instructions' own: the Exception list puts the
/// repayment case above the exclusion case, and a filer whose benefits are entirely non-taxable does not
/// need to answer a question about which worksheet to use.
pub fn screen(
    rows: &[FormSsa1099],
    status: crate::tax::FilingStatus,
    mfs_lived_apart: Option<bool>,
    has_exclusion: Option<bool>,
) -> Result<(), SsRefusal> {
    if rows.is_empty() {
        return Ok(());
    }
    let paid: Usd = rows.iter().map(|f| f.box3_benefits_paid).sum();
    let repaid: Usd = rows.iter().map(|f| f.box4_benefits_repaid).sum();
    if repaid > paid {
        return Err(SsRefusal::RepaymentsExceedBenefits {
            excess: repaid - paid,
        });
    }
    match has_exclusion {
        Some(true) => return Err(SsRefusal::WorksheetBarredByExclusion),
        None => return Err(SsRefusal::ExclusionUnanswered),
        Some(false) => {}
    }
    // ★★★ R-8, LAST of the three, and the order is argued: a filer whose benefits are wholly
    //     non-taxable (R-6) or who must use a different worksheet entirely (R-7) does not need to settle
    //     a question that only changes which threshold this worksheet applies.
    if status == crate::tax::FilingStatus::Mfs && mfs_lived_apart.is_none() {
        return Err(SsRefusal::MfsLivedApartUnanswered);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ssa(owner: Owner, paid: i64, repaid: i64, withheld: i64) -> FormSsa1099 {
        FormSsa1099 {
            owner,
            kind: SsaFormKind::Ssa1099,
            transcribed_on: None,
            box3_benefits_paid: rust_decimal::Decimal::from(paid),
            box4_benefits_repaid: rust_decimal::Decimal::from(repaid),
            federal_withholding: rust_decimal::Decimal::from(withheld),
        }
    }

    /// ★★★ **The two forms report withholding in DIFFERENT boxes, and box 6 means something else
    /// entirely on the RRB form.** Asserted against the archived facsimiles, so a revision that moved
    /// either box reds rather than being absorbed.
    #[test]
    fn the_withholding_box_differs_between_the_two_forms_and_the_archive_says_so() {
        assert_eq!(SsaFormKind::Ssa1099.withholding_box(), "6");
        assert_eq!(SsaFormKind::Rrb1099.withholding_box(), "10");
        let pub915 = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(
                    "legal/text/irs-publications/Pub915_Social_Security_and_RRB_Benefits--2025.txt",
                ),
        )
        .expect("the archived Pub. 915");
        assert!(
            pub915.contains("Box 6. Voluntary Federal Income Tax Withheld"),
            "SSA-1099's box 6 IS the withholding"
        );
        assert!(
            pub915.contains("Box 10—Federal Income Tax Withheld"),
            "RRB-1099's box 10 is the withholding"
        );
        assert!(
            pub915.contains("Box 6—Workers’ Compensation Offset"),
            "and RRB-1099's box 6 is something else entirely — which is why no field here is named \
             `box6_`"
        );
    }

    /// ★★★ **Box 5 is derived and SIGNED, and the archive prints its definition.**
    #[test]
    fn box_5_is_box_3_minus_box_4_and_goes_negative() {
        assert_eq!(
            box5_net_benefits(&ssa(Owner::Taxpayer, 12_000, 0, 0)),
            dec!(12_000)
        );
        assert_eq!(
            box5_net_benefits(&ssa(Owner::Taxpayer, 12_000, 2_000, 0)),
            dec!(10_000)
        );
        let negative = box5_net_benefits(&ssa(Owner::Taxpayer, 1_000, 3_500, 0));
        assert_eq!(
            negative,
            dec!(-2_500),
            "the parenthesised case the form prints"
        );
        assert!(negative < Usd::ZERO);
    }

    /// ★★★ **PUB 915'S OWN WORKED EXAMPLE, as a KAT: Ryan $3,000 and Jordan ($500) net to $2,500.**
    ///
    /// This is the case a per-spouse or per-form R-6 would get wrong — it would see Jordan's negative
    /// form and refuse a return the instructions finish in one sentence. A refusal that is too WIDE
    /// costs the filer their return, which is the direction that matters here.
    #[test]
    fn pub915s_ryan_and_jordan_example_nets_to_2500_and_does_not_refuse() {
        let rows = vec![
            ssa(Owner::Taxpayer, 3_000, 0, 0), // Ryan: box 5 = 3,000
            ssa(Owner::Spouse, 0, 500, 0),     // Jordan: box 5 = (500)
        ];
        assert_eq!(worksheet_line1(&rows), dec!(2_500), "Pub. 915's own figure");
        assert_eq!(
            screen(&rows, crate::tax::FilingStatus::Single, None, Some(false)),
            Ok(()),
            "Σ box 4 (500) does not exceed Σ box 3 (3,000), so R-6 must NOT fire — a per-form test \
             would refuse Jordan's form and turn the household away"
        );
    }

    /// ★★ **R-6 fires on the SUM across both spouses, and carries the §1341 excess.**
    #[test]
    fn r6_fires_on_the_cross_spouse_sum_and_names_the_excess() {
        // Σ box 3 = 1,000; Σ box 4 = 9,000 ⇒ excess 8,000, above the §1341 threshold.
        let rows = vec![
            ssa(Owner::Taxpayer, 1_000, 4_000, 0),
            ssa(Owner::Spouse, 0, 5_000, 0),
        ];
        assert_eq!(
            screen(&rows, crate::tax::FilingStatus::Single, None, Some(false)),
            Err(SsRefusal::RepaymentsExceedBenefits {
                excess: dec!(8_000)
            })
        );
        assert!(
            dec!(8_000) > SECTION_1341_THRESHOLD,
            "the §1341 routes are available here"
        );
        // ★ And EXACT equality does not refuse: the instructions say repayments must be MORE than
        //   benefits, and refusing at equality would be the too-wide direction again.
        let equal = vec![ssa(Owner::Taxpayer, 5_000, 5_000, 0)];
        assert_eq!(
            screen(&equal, crate::tax::FilingStatus::Single, None, Some(false)),
            Ok(())
        );
        assert_eq!(worksheet_line1(&equal), Usd::ZERO);
    }

    /// ★★★ **R-7 — the worksheet is BARRED by the Exception, and its silence REFUSES.**
    ///
    /// Guessing `false` would run a worksheet the instructions forbid for this filer and produce a
    /// figure Pub. 915 says must come from a different worksheet entirely.
    #[test]
    fn r7_bars_the_worksheet_and_silence_refuses() {
        let rows = vec![ssa(Owner::Taxpayer, 20_000, 0, 0)];
        assert_eq!(
            screen(&rows, crate::tax::FilingStatus::Single, None, Some(true)),
            Err(SsRefusal::WorksheetBarredByExclusion)
        );
        assert_eq!(
            screen(&rows, crate::tax::FilingStatus::Single, None, None),
            Err(SsRefusal::ExclusionUnanswered)
        );
        assert_eq!(
            screen(&rows, crate::tax::FilingStatus::Single, None, Some(false)),
            Ok(())
        );
        // ★ With NO forms at all nothing is asked: the question is live only because a benefit exists.
        assert_eq!(
            screen(&[], crate::tax::FilingStatus::Single, None, None),
            Ok(()),
            "no benefits ⇒ no question, no refusal"
        );
    }

    /// ★★ **R-6 is tested BEFORE R-7**, matching the Exception list's own order: a filer whose benefits
    /// are entirely non-taxable does not need to answer which worksheet to use.
    #[test]
    fn r6_precedes_r7() {
        let repaid_more = vec![ssa(Owner::Taxpayer, 1_000, 5_000, 0)];
        for exclusion in [None, Some(true), Some(false)] {
            assert!(
                matches!(
                    screen(
                        &repaid_more,
                        crate::tax::FilingStatus::Single,
                        None,
                        exclusion
                    ),
                    Err(SsRefusal::RepaymentsExceedBenefits { .. })
                ),
                "R-6 must fire whatever the exclusion answer is ({exclusion:?})"
            );
        }
    }

    /// ★ Withholding sums across both forms and both spouses → 1040 line 25b.
    #[test]
    fn withholding_sums_across_forms_and_spouses() {
        let rows = vec![
            ssa(Owner::Taxpayer, 20_000, 0, 1_800),
            FormSsa1099 {
                kind: SsaFormKind::Rrb1099,
                ..ssa(Owner::Spouse, 14_000, 0, 900)
            },
        ];
        assert_eq!(withholding(&rows), dec!(2_700));
        assert_eq!(withholding(&[]), Usd::ZERO);
    }
}
