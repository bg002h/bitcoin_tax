//! Acceptance vectors transcribed from a **published** TY2024 return corpus — the third witness the
//! retirement feature had none of.
//!
//! ## Why this module exists
//!
//! `FOLLOWUPS.md` FR-250: the 107-household golden corpus contains **no retirement income at all**.
//! `SPEC_retirement_income.md` §9a (review r1, C-3): on the MFS-lived-with branch the two oracles
//! **agree while both wrong** — taxcalc's own docstring says it *"unconditionally treats every MARS=3
//! filer as lived-apart"* and OTS has no lived-with branch either. So Form 1040 lines 4a–6b had **no
//! independent witness**: the oracles cannot witness the worksheet's hardest branch, and the corpus
//! cannot witness the feature at all.
//!
//! These vectors are one. They come from **TaxCalcBench**, `github.com/column-tax/tax-calc-bench`,
//! **MIT licensed**, 51 complete TY2024 returns each carrying expected MeF output.
//!
//! ## Provenance — pinned, hashed, and re-fetchable
//!
//! Every figure below was read from `output.xml` at corpus commit
//! `8f89c2cf00a8906f4d896a02a2f45f9c9e85ae9b` (2026-09-08). The raw data is **not** committed here —
//! ~46 MB fits neither archive convention in this repo (publications are committed, form PDFs are
//! gitignored), so it is fetch-on-demand against the pinned SHA, which is the shape
//! `authority-refresh` already uses for IRS documents:
//!
//! ```text
//! B=https://raw.githubusercontent.com/column-tax/tax-calc-bench/8f89c2cf00a8906f4d896a02a2f45f9c9e85ae9b/tax_calc_bench/ty24/test_data
//! curl -sL "$B/<case>/output.xml" | sha256sum   # must equal Vector::output_sha256_prefix
//! ```
//!
//! ★★ **A published corpus is a WITNESS, never an AUTHORITY.** When one of these disagrees with
//! btctax, adjudicate against the **form**. The precedent is tenforty #278/#279: OTS was never wrong,
//! the wrapper was. A vector that cannot be reconciled to the form's own words is a finding about the
//! vector.
//!
//! ## What these vectors deliberately do NOT assert
//!
//! * **Line 4b is absent from the corpus XML.** `single-retirement-1099r-alaska-dividend` carries 4a,
//!   5a, 5b, line 9 and AGI — and no taxable-IRA element. Its 4b is **DERIVED** from line 9, and
//!   [`Vector::line9_reconciles`] forces that derivation to stay arithmetically true rather than
//!   leaving it as a comment somebody can edit away.
//! * **No MFS case exists in the corpus**, so §9a's zero-witness branch stays at zero. This module
//!   does not close it, and must not be cited as if it did.
//! * **No charitable and no HSA case**, so FR-250's gap is untouched.
//! * **TY2024 only.** The owner ruled both years; TY2025 vectors must come from elsewhere.
//! * These are synthetic-but-verified returns, not **filed** ones. They do not substitute for FR-64,
//!   the owner's own documents driven end to end.

use crate::conventions::Usd;
use rust_decimal_macros::dec;

/// One published return's expected Form 1040 figures, as read from its `output.xml`.
///
/// ★ Every field is what the corpus **asserts**, except [`Self::taxable_ira_4b_derived`] — see the
/// module note and [`Vector::line9_reconciles`].
#[derive(Debug, Clone, Copy)]
pub struct Vector {
    /// The corpus's own directory name, which is its identity upstream.
    pub case: &'static str,
    /// First 16 hex of `sha256(output.xml)` at the pinned corpus commit. A changed corpus revision
    /// shows up here rather than silently altering an expected figure.
    pub output_sha256_prefix: &'static str,
    /// 1040 **4a** — `IRADistributionsAmt`. `None` when the case has no IRA distribution.
    pub ira_distributions_4a: Option<Usd>,
    /// 1040 **4b** — ★ **DERIVED, NOT ASSERTED.** The corpus prints no taxable-IRA element; this is
    /// line 9 minus the other income components. `None` when there is no IRA distribution at all.
    pub taxable_ira_4b_derived: Option<Usd>,
    /// 1040 **5a** — `PensionsAnnuitiesAmt`.
    pub pensions_annuities_5a: Option<Usd>,
    /// 1040 **5b** — `TotalTaxablePensionsAmt`.
    pub taxable_pensions_5b: Option<Usd>,
    /// 1040 **6a** — `SocSecBnftAmt`.
    pub social_security_6a: Option<Usd>,
    /// 1040 **6b** — the taxable portion.
    ///
    /// ★★★ **`None` means the corpus emitted NO taxable-benefit element**, which is not the same as
    /// `Some(0)` and is the whole reason this field is an `Option`. Both non-taxable cases below are
    /// `None`: an independent MeF corpus represents a non-taxable benefit as the **absence of
    /// testimony**, exactly as `CLAUDE.md`'s *"blank is the normal case"* requires of btctax. Storing
    /// a zero here would erase the finding.
    pub taxable_social_security_6b: Option<Usd>,
    /// Everything else on line 9 that is not 4b/5b/6b — Schedule 1 line 10, wages, interest and so on.
    /// Exists so [`Vector::line9_reconciles`] can force 4b's derivation.
    pub other_line9_components: Usd,
    /// 1040 **9** — `TotalIncomeAmt`.
    pub total_income_9: Usd,
    /// 1040 **11** — `AdjustedGrossIncomeAmt`.
    pub agi_11: Usd,
    /// 1040 **12** — `TotalItemizedOrStandardDedAmt`.
    pub deduction_12: Usd,
    /// 1040 **25b** — `WithholdingTaxAmt`.
    ///
    /// ★★ This is the field that corroborates review r1's C-2 from outside this repo: the 1099-R case
    /// asserts **3,000**, exactly its two Form 1099-R box-4 amounts (1,000 + 2,000). btctax's
    /// `withholding_25b` sums *"Σ box 4, across INT/DIV/G"* and would drop both.
    pub withholding_25b: Option<Usd>,
}

impl Vector {
    /// Does line 9 equal `4b + 5b + 6b + everything else`?
    ///
    /// ★★★ **This is what makes 4b's derivation load-bearing rather than a comment.** The corpus never
    /// prints 4b, so it is computed here; if a future edit changes any component without changing line
    /// 9, this stops holding and the vector is rejected. A derived figure nobody re-derives is a
    /// fabricated KAT.
    #[must_use]
    pub fn line9_reconciles(&self) -> bool {
        let z = Usd::ZERO;
        self.taxable_ira_4b_derived.unwrap_or(z)
            + self.taxable_pensions_5b.unwrap_or(z)
            + self.taxable_social_security_6b.unwrap_or(z)
            + self.other_line9_components
            == self.total_income_9
    }
}

/// The four genuine 1099-R / SSA-1099 cases among TaxCalcBench's 51.
///
/// ★ The other four retirement-matching cases are *excess Social Security tax* withholding (a Schedule
/// 3 credit), not retirement income, and are deliberately absent: including them would inflate the
/// apparent coverage of lines 4a–6b.
pub const PUBLIC_RETIREMENT_VECTORS: &[Vector] = &[
    // A 65-or-older single filer: deduction 16,550 = 14,600 basic + 1,950 §63(f) aged addition.
    // Two Forms 1099-R (10,000 IRA and 20,000 pension, both fully taxable) plus a 1,000 Alaska
    // Permanent Fund dividend on Schedule 1.
    Vector {
        case: "single-retirement-1099r-alaska-dividend",
        output_sha256_prefix: "e56cbe5298ab1b54",
        ira_distributions_4a: Some(dec!(10000)),
        taxable_ira_4b_derived: Some(dec!(10000)),
        pensions_annuities_5a: Some(dec!(20000)),
        taxable_pensions_5b: Some(dec!(20000)),
        social_security_6a: None,
        taxable_social_security_6b: None,
        other_line9_components: dec!(1000), // the Alaska dividend, via Schedule 1 line 10
        total_income_9: dec!(31000),
        agi_11: dec!(31000),
        deduction_12: dec!(16550),
        withholding_25b: Some(dec!(3000)),
    },
    // ★★ A 65-or-older head of household (deduction 23,850 = 21,900 + 1,950) with 8,742 of benefits
    //    and NO taxable-benefit element at all — the worksheet's "none of your benefits are taxable"
    //    STOP, represented as a blank rather than a zero.
    Vector {
        case: "hoh-schedule-b-ssa1099-unemployment",
        output_sha256_prefix: "54202d0fa67aaccd",
        ira_distributions_4a: None,
        taxable_ira_4b_derived: None,
        pensions_annuities_5a: None,
        taxable_pensions_5b: None,
        social_security_6a: Some(dec!(8742)),
        taxable_social_security_6b: None,
        other_line9_components: dec!(27038),
        total_income_9: dec!(27038),
        agi_11: dec!(26447),
        deduction_12: dec!(23850),
        withholding_25b: Some(dec!(17)),
    },
    // ★★ Married filing jointly, both blind: deduction 35,400 = 29,200 basic + 2 × 1,550 × 2 boxes.
    //    7,333 of benefits, again with NO taxable-benefit element.
    Vector {
        case: "mfj-both-blind-nontaxable-social-security",
        output_sha256_prefix: "480c853e471d7570",
        ira_distributions_4a: None,
        taxable_ira_4b_derived: None,
        pensions_annuities_5a: None,
        taxable_pensions_5b: None,
        social_security_6a: Some(dec!(7333)),
        taxable_social_security_6b: None,
        other_line9_components: dec!(5000),
        total_income_9: dec!(5000),
        agi_11: dec!(5000),
        deduction_12: dec!(35400),
        withholding_25b: Some(dec!(1000)),
    },
    // A minimal case kept for its deduction line: single, basic 14,600, everything else near zero.
    Vector {
        case: "single-w2-retirement-sick-pay-social-security-tip",
        output_sha256_prefix: "7ff4f7b0fef30c84",
        ira_distributions_4a: None,
        taxable_ira_4b_derived: None,
        pensions_annuities_5a: None,
        taxable_pensions_5b: None,
        social_security_6a: None,
        taxable_social_security_6b: None,
        other_line9_components: dec!(100),
        total_income_9: dec!(100),
        agi_11: dec!(100),
        deduction_12: dec!(14600),
        withholding_25b: None,
    },
];

/// The first Form 1040 **retirement** line declared in a `Form1040Lines` struct body, or `None`.
///
/// ★★★ Split out of the test below so the check is **plantable**. The obvious test — add `line4b` to the
/// real struct and watch it red — cannot be run: adding a field to `Form1040Lines` is an `E0063` at every
/// construction site, so the crate stops compiling before any test executes. The compiler is therefore the
/// real forcing function, and this is the reminder on top of it. But a guard that cannot be observed
/// discriminating is exactly what `design/HARNESS.md` B1 forbids, so the predicate takes a string and the
/// test plants a fixture.
#[must_use]
pub fn first_retirement_line_declared(struct_body: &str) -> Option<&'static str> {
    [
        "pub line4a:",
        "pub line4b:",
        "pub line5a:",
        "pub line5b:",
        "pub line6a:",
        "pub line6b:",
    ]
    .into_iter()
    .find(|f| struct_body.contains(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **The derivation of line 4b is forced, not asserted in prose.** The corpus prints no
    /// taxable-IRA element, so 4b is computed from line 9; this test is what stops that computation
    /// from drifting into a made-up number. Edit any component without editing line 9 and it reds.
    #[test]
    fn every_vector_reconciles_line_9_so_the_derived_4b_cannot_drift() {
        assert!(
            !PUBLIC_RETIREMENT_VECTORS.is_empty(),
            "an empty vector table makes every assertion below vacuous"
        );
        for v in PUBLIC_RETIREMENT_VECTORS {
            assert!(
                v.line9_reconciles(),
                "{}: 4b + 5b + 6b + other != line 9. The corpus does not print 4b, so it is DERIVED — \
                 if this no longer holds the derivation is wrong and the vector is a fabricated KAT",
                v.case
            );
        }
    }

    /// ★★★ **A non-taxable benefit is a BLANK, not a zero — and the corpus says so independently.**
    ///
    /// Both SSA-1099 cases carry 6a with **no** taxable-benefit element in their `output.xml`. That is
    /// an unrelated MeF corpus representing the absence of testimony exactly as `CLAUDE.md` requires of
    /// btctax. This test exists so a future author cannot "tidy" those `None`s into `Some(0)` and erase
    /// the finding — which would also make the vector assert a figure the corpus never printed.
    #[test]
    fn a_non_taxable_benefit_is_none_and_never_some_zero() {
        let with_benefits: Vec<&Vector> = PUBLIC_RETIREMENT_VECTORS
            .iter()
            .filter(|v| v.social_security_6a.is_some())
            .collect();
        assert_eq!(
            with_benefits.len(),
            2,
            "premise: exactly two vectors carry Social Security benefits. If this moved, the rest of \
             this test is measuring something else"
        );
        for v in with_benefits {
            assert_eq!(
                v.taxable_social_security_6b, None,
                "{}: the corpus printed NO taxable-benefit element for this case. `Some(Usd::ZERO)` \
                 would assert a figure it never printed, and would erase the very finding this vector \
                 was transcribed to record",
                v.case
            );
        }
    }

    /// ★★ Provenance must be checkable, or "pinned and hashed" is a claim rather than a property.
    #[test]
    fn every_vector_carries_a_checkable_provenance() {
        for v in PUBLIC_RETIREMENT_VECTORS {
            assert!(!v.case.is_empty(), "a vector with no upstream case name");
            assert_eq!(
                v.output_sha256_prefix.len(),
                16,
                "{}: the hash prefix must be 16 hex so a corpus revision is detectable",
                v.case
            );
            assert!(
                v.output_sha256_prefix
                    .chars()
                    .all(|c| c.is_ascii_hexdigit()),
                "{}: non-hex in the sha256 prefix",
                v.case
            );
        }
        let n = PUBLIC_RETIREMENT_VECTORS.len();
        let uniq: std::collections::BTreeSet<&str> =
            PUBLIC_RETIREMENT_VECTORS.iter().map(|v| v.case).collect();
        assert_eq!(uniq.len(), n, "two vectors share an upstream case name");
    }

    /// ★★★ **THE TRIPWIRE. These vectors are not consumed by anything yet, and that is correct today —
    /// Form 1040 lines 4a–6b do not exist. This test reds the moment they do.**
    ///
    /// Without it the vectors are decorative: a table of correct numbers that no computation reads,
    /// which is `a-figure-with-no-reader` in reverse. When someone adds `line4b` to the printed 1040,
    /// this fails and its message says what to do — wire the vectors into a KAT, then delete this test.
    ///
    /// ★ It is derived from the struct's own source rather than from a list of field names, so it
    /// cannot go stale against a rename.
    #[test]
    fn when_the_1040_gains_a_retirement_line_these_vectors_must_be_wired_in() {
        let printed = include_str!("printed.rs");
        let i = printed
            .find("pub struct Form1040Lines")
            .expect("Form1040Lines is declared in printed.rs");
        let j = printed[i..].find("\n}").expect("the struct closes") + i;
        let body = &printed[i..j];
        assert_eq!(
            first_retirement_line_declared(body),
            None,
            "`Form1040Lines` now declares a retirement line — so the retirement lines exist. \
             crates/btctax-core/src/tax/public_vectors.rs holds four PUBLISHED acceptance vectors that \
             nothing reads. Wire them into a KAT that drives each case and compares the printed lines, \
             then delete this test. ★ Note 4b is DERIVED, not asserted by the corpus — carry that \
             derivation into the KAT."
        );

        // ★★ B1 — the predicate observed discriminating, on fixtures, because the real plant cannot be
        //    run: adding a field to `Form1040Lines` is an E0063 at every construction site and the crate
        //    stops compiling before any test executes.
        assert_eq!(
            first_retirement_line_declared("    pub line1z: Usd,\n    pub line4b: Usd,\n"),
            Some("pub line4b:"),
            "a declared retirement line must be found — otherwise this tripwire is decorative"
        );
        assert_eq!(
            first_retirement_line_declared("    pub line1z: Usd,\n    pub line7: Usd,\n"),
            None,
            "and a body with no retirement line must not be flagged, or the tripwire reds on every \
             1040 forever and gets deleted"
        );
        // ★ The near miss that would make a substring check wrong: `line4b` must not be found inside a
        //   longer identifier or a comment mentioning it.
        assert_eq!(
            first_retirement_line_declared("    /// see line4b later\n    pub line7: Usd,\n"),
            None,
            "a COMMENT mentioning line4b is not a declaration"
        );
    }
}
