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
    CtcOdcOmitted { dependents: usize },
    /// §3.4 conservative omission: the Earned Income Credit is not computed and the household's income is
    /// low enough that it might qualify. Overstates tax.
    EicOmitted,
    /// §63(f): a person's date of birth is not on file, so the aged (65+) additional standard deduction is
    /// NOT granted (never granted on an unsubstantiated birthdate). Overstates tax if they are 65+.
    AgedBoxForfeitedNoDob { per_box: Usd },
    /// FinCEN Notice 2020-2 disclosure — the filer declared a foreign financial account. v1 never
    /// auto-answers Schedule B Part III.
    FbarFinCen,
    /// The ledger classified crypto donations assuming a **public charity (50%-org)** donee. A private
    /// foundation is the 20%-ceiling / basis class (which v1 refuses), so the donee must be verified.
    CharitableDoneeAssumedPublicCharity { donations: usize },
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
    /// `Some(false)`. `persons` counts the taxpayer's box plus, ON MFJ ONLY, the spouse's (an absent MFJ
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
}

/// Format a dollar amount for advisory prose: `$1,950` / `$1,234.56` — thousands-separated, and
/// cents shown only when there are any. [★ P5-N5] The advisories used a bare `{:.0}` (`$1950`),
/// which disagreed with the comma-separated house style every other printed figure uses. The CLI's
/// `fmt_money` lives in `btctax-cli::render` and core cannot reach it, so this is the core-side
/// equivalent — deliberately small, and used by every advisory that prints money.
fn fmt_usd(v: Usd) -> String {
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
            Advisory::CtcOdcOmitted { dependents } => format!(
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
            Advisory::FbarFinCen =>
                "FBAR / FinCEN — you declared a foreign financial account. Under FinCEN Notice 2020-2 an \
                 account holding ONLY virtual currency is (for now) outside the FBAR requirement, but that \
                 is under active reconsideration, and an account holding crypto PLUS fiat or securities may \
                 well be reportable. btctax never answers Schedule B Part III for you — decide it yourself."
                    .to_string(),
            Advisory::CharitableDoneeAssumedPublicCharity { donations } => format!(
                "CHARITABLE DONEE ASSUMED — your {donations} crypto donation(s) were valued assuming a \
                 PUBLIC CHARITY (50%-organization) donee: long-term gifts at fair market value under the \
                 30%-of-AGI ceiling. If the donee is a PRIVATE FOUNDATION, the correct treatment is the \
                 20% ceiling at BASIS (which v1 refuses). Verify who you gave to."
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
        }
    }
}

/// Collect every advisory that applies to a computed full return for `year`, from the assembled return.
pub fn advisories_for(
    ri: &ReturnInputs,
    state: &LedgerState,
    ar: &AbsoluteReturn,
    params: &FullReturnParams,
    year: i32,
) -> Vec<Advisory> {
    let earned = ar.wages + ar.se.as_ref().map_or(Usd::ZERO, |s| s.net_se);
    advisories(
        ri,
        state,
        earned,
        ar.agi,
        ar.overpayment_refund,
        params,
        year,
        ar.deduction_is_itemized,
    )
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
        out.push(Advisory::CtcOdcOmitted { dependents });
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
    let spouse_no_dob = ri.filing_status == FilingStatus::Mfj && !spouse_dob_on_file;
    if taxpayer_no_dob || spouse_no_dob {
        out.push(Advisory::AgedBoxForfeitedNoDob { per_box });
    }

    // ★ §63(f) BLINDNESS forgone (P9 §2.2) — same statute, rate and worksheet line as the aged box, and it
    // STACKS. Fires on `blind.is_none()` (never asked), never on `Some(false)`, counting the spouse box only
    // on MFJ (an ABSENT MFJ spouse forfeits too; MFS never counts the spouse — mirrors `AgedBlindBoxes`).
    let taxpayer_no_blind = ri.header.taxpayer.blind.is_none();
    let spouse_blind_on_file = ri.header.spouse.as_ref().is_some_and(|s| s.blind.is_some());
    let spouse_no_blind = ri.filing_status == FilingStatus::Mfj && !spouse_blind_on_file;
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

    // FinCEN Notice 2020-2 — a declared foreign account.
    if ri.foreign_accounts == Some(true) {
        out.push(Advisory::FbarFinCen);
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

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{Dependent, ScheduleAInputs};

    /// The advisory set is driven by the inputs/ledger, and every message names the direction of the error
    /// (OVERSTATED) so a filer knows the omission costs them money, not the IRS.
    #[test]
    fn ctc_fires_on_dependents_and_says_overstated() {
        let a = Advisory::CtcOdcOmitted { dependents: 2 };
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

    /// A high-income Single filer WITH a DOB, no dependents, no foreign account, no donations gets
    /// exactly ONE advisory — the unconditional other-credits omission — and no spurious ones.
    ///
    /// [★ P5-I2] This used to assert NO advisories. That was wrong, not merely untested: the
    /// residential-energy credit has no income limit at all, and the adoption credit reaches into the
    /// $250k band, so "v1 did not compute these" applies to every return ever produced. The test's
    /// real intent — the common case must not be NOISY — is preserved by pinning the exact set.
    #[test]
    fn a_clean_high_income_return_has_only_the_unconditional_omission() {
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
        assert_eq!(got, vec![Advisory::OtherCreditsOmitted], "{got:?}");
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
                Advisory::CtcOdcOmitted { dependents: 2 },
                Advisory::EicOmitted,
                Advisory::OtherCreditsOmitted,
                Advisory::AgedBoxForfeitedNoDob {
                    per_box: dec!(1950)
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
