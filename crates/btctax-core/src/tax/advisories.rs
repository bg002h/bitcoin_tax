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
use crate::tax::return_1040::AbsoluteReturn;
use crate::tax::return_inputs::ReturnInputs;
use crate::tax::tables::FullReturnParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// The AGI ceiling below which the **EIC** advisory fires. Deliberately a round over-estimate of the
/// TY2024 maximum EIC AGI (≈ $59,899 for MFJ with 3+ children) — this only decides whether to SHOW a
/// "you may qualify" note, never a computed figure, so over-firing is the safe direction.
const EIC_ADVISORY_AGI_CEILING: Usd = dec!(60000);

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
                 (${per_box:.0} per box) was NOT granted, because v1 never assumes a birthdate. If you (or \
                 your spouse) are 65 or older, enter the date of birth and re-run: your tax is currently \
                 OVERSTATED."
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
    advisories(ri, state, earned, ar.agi, params, year)
}

/// Collect every advisory that applies (the scalar form — `earned_income` = wages + net SE earnings;
/// `agi` = 1040 L11). Order is stable: omissions first (they cost the filer money), then disclosures.
pub fn advisories(
    ri: &ReturnInputs,
    state: &LedgerState,
    earned_income: Usd,
    agi: Usd,
    params: &FullReturnParams,
    year: i32,
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
    let spouse_no_dob = ri.filing_status == FilingStatus::Mfj
        && ri
            .header
            .spouse
            .as_ref()
            .is_some_and(|s| s.date_of_birth.is_none());
    if taxpayer_no_dob || spouse_no_dob {
        out.push(Advisory::AgedBoxForfeitedNoDob { per_box });
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
    use crate::tax::return_inputs::Dependent;

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
        assert!(m.contains("$1950") || m.contains("1950"));
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

    /// A high-income Single filer WITH a DOB, no dependents, no foreign account, no donations → no
    /// advisories at all (the common case must not be noisy).
    #[test]
    fn a_clean_high_income_return_has_no_advisories() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 01 - 01));
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(150000), // earned
            dec!(150000), // AGI — well over the EIC ceiling
            &params(),
            2024,
        );
        assert!(got.is_empty(), "{got:?}");
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
        // taxpayer.date_of_birth defaults to None → aged box forfeited.
        let got = advisories(
            &ri,
            &LedgerState::default(),
            dec!(30000), // earned
            dec!(30000), // AGI < the $60k EIC advisory ceiling
            &params(),
            2024,
        );
        assert_eq!(
            got,
            vec![
                Advisory::CtcOdcOmitted { dependents: 2 },
                Advisory::EicOmitted,
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
            dec!(30000),
            &p,
            2024
        )
        .contains(&Advisory::EicOmitted));
        // Earned income but AGI at/over the ceiling → no EIC advisory.
        assert!(!advisories(
            &ri,
            &LedgerState::default(),
            dec!(60000),
            dec!(60000),
            &p,
            2024
        )
        .contains(&Advisory::EicOmitted));
        // Earned income + low AGI → fires.
        assert!(advisories(
            &ri,
            &LedgerState::default(),
            dec!(30000),
            dec!(30000),
            &p,
            2024
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
            &params(),
            2024,
        );
        assert!(got.contains(&Advisory::FbarFinCen));
        assert!(got.contains(&Advisory::AgedBoxForfeitedNoDob {
            per_box: dec!(1550) // married rate
        }));
    }
}
