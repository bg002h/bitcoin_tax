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
    /// ★★★ **The filing status, added 2026-09-20 — and its absence is why the §86 defect below went
    ///        unnoticed at step 1.** Every threshold in §86(c) is per status, so a vector that cannot
    ///        state its status cannot be reconciled against the statute at all; it could only be
    ///        transcribed and trusted. Read from the corpus's `deduction_12` and its case name.
    pub filing_status: crate::tax::FilingStatus,
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

    /// ★★★ **DOES THIS VECTOR'S LINE 6b AGREE WITH §86, computed from its own figures?**
    ///
    /// Returns `Ok(())`, or the taxable benefit the statute gives. This is the check step 1 could not
    /// perform — [`Self::filing_status`] did not exist — and it is the check that matters most, because
    /// 6b is the line the retirement feature computes and these vectors are its only independent
    /// witness. A transcription that is merely *read carefully* is not a witness; one that reconciles
    /// against the statute is.
    ///
    /// ★★ The `other_line9_components` total stands in for worksheet line 3, which is legitimate for
    /// these four cases and is asserted rather than assumed: line 3 combines 1040 lines 1z, 2b, 3b, 4b,
    /// 5b, 7 and 8, and every non-retirement component of these cases (wages, interest, dividends,
    /// unemployment via Schedule 1 → line 8, the Alaska dividend likewise) lands on one of them. A
    /// future vector with income OUTSIDE that set would need line 3 stated separately.
    pub fn social_security_reconciles(&self) -> Result<(), Usd> {
        let Some(benefits) = self.social_security_6a else {
            return Ok(());
        };
        let statutory = crate::tax::ss_benefits_worksheet::run(
            benefits,
            self.filing_status,
            // No MFS case exists in this corpus, so the lived-apart answer is never read.
            false,
            crate::tax::ss_benefits_worksheet::OtherIncome {
                line3_combined: self.other_line9_components,
                line4_tax_exempt_interest: Usd::ZERO,
                line6_schedule_1_block: Usd::ZERO,
            },
        )
        .line_6b();
        let asserted = self.taxable_social_security_6b.unwrap_or(Usd::ZERO);
        if statutory == asserted {
            Ok(())
        } else {
            Err(statutory)
        }
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
        // deduction 16,550 = 14,600 basic + one 1,950 §63(f) aged addition ⇒ single, 65 or older.
        filing_status: crate::tax::FilingStatus::Single,
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
        // deduction 23,850 = 21,900 + 1,950 ⇒ head of household, 65 or older; the case name says so too.
        filing_status: crate::tax::FilingStatus::HoH,
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
        // deduction 35,400 = 29,200 + 2 × 1,550 × 2 boxes ⇒ married filing jointly, both blind.
        filing_status: crate::tax::FilingStatus::Mfj,
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
        // deduction 14,600, the TY2024 single basic amount with no addition.
        filing_status: crate::tax::FilingStatus::Single,
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

    /// ★★★ **EVERY VECTOR'S LINE 6b RECONCILES WITH §86, COMPUTED FROM ITS OWN FIGURES — and one
    ///        does not, which is why this test names it.**
    ///
    /// Step 1 transcribed these vectors carefully and could not do this check, because [`Vector`] had
    /// no filing status and every §86(c) threshold is per status. So a vector could be *read correctly*
    /// and still be arithmetically impossible, and one is.
    ///
    /// ★★★ **`hoh-schedule-b-ssa1099-unemployment` is DISPUTED.** Head of household, **$8,742** of
    /// benefits, **$27,038** of other income. §86(c)(1)(A) gives a head of household the $25,000 base
    /// amount, so provisional income is 27,038 + 4,371 = **31,409**, the worksheet does not stop, and
    /// the taxable benefit is **$3,204.50**. The corpus's `output.xml` emits **no** taxable-benefit
    /// element, and its line 9 of 27,038 is self-consistent with zero.
    ///
    /// **THREE INDEPENDENT WITNESSES give 3,204.50** (2026-09-20):
    ///
    /// | witness | line 6b |
    /// |---|---|
    /// | §86 worked by hand from the statute | 3,204.50 |
    /// | `ss_benefits_worksheet::run` (this repo, transcribed from `i1040gi`) | 3,204.50 |
    /// | Tax-Calculator 6.8.2, `c02500` | 3,204.50 |
    ///
    /// ★★ And taxcalc's AGI for the case is **30,242.50 = 27,038 + 3,204.50 exactly**, which is the
    /// corpus's own other-income total plus the benefit it does not print. That is not a rounding
    /// disagreement; it is a missing element.
    ///
    /// **So the vector may NOT be used as a 6b witness**, and the module note's claim that both
    /// SSA-1099 cases show "a non-taxable benefit as the absence of testimony" holds only for
    /// `mfj-both-blind-…`, which genuinely stops at worksheet line 9 (all three witnesses agree).
    /// `CLAUDE.md`: *"A disagreement is adjudicated against the FORM, never encoded."* The form says
    /// 3,204.50. Nothing is filed upstream on this yet — the corpus's `input.json` has not been read,
    /// so it cannot be ruled out that the case's actual inputs differ from the transcribed totals, and
    /// the standing rule is never to file on one reading. `FOLLOWUPS.md` FR-255 carries it.
    #[test]
    fn every_vector_with_benefits_reconciles_with_section_86_or_is_named_here() {
        /// Cases whose 6b contradicts §86, with the statutory figure and why they are still committed.
        /// ★ Fail-closed: a NEW disagreement is not on this list, so it reds.
        const DISPUTED: &[(&str, &str)] = &[(
            "hoh-schedule-b-ssa1099-unemployment",
            "the corpus emits no taxable-benefit element; §86 by hand, this repo's worksheet and \
             taxcalc 6.8.2 all give 3,204.50, and taxcalc's AGI is the corpus's line 9 plus exactly \
             that. Unusable as a 6b witness until the corpus input.json is read (FR-255).",
        )];
        let mut carried = 0;
        for v in PUBLIC_RETIREMENT_VECTORS {
            match v.social_security_reconciles() {
                Ok(()) => assert!(
                    !DISPUTED.iter().any(|(c, _)| *c == v.case),
                    "{} is listed as DISPUTED and now reconciles. If the vector was corrected, remove \
                     the entry in the same commit — a stale dispute discredits the list.",
                    v.case
                ),
                Err(statutory) => {
                    let why = DISPUTED.iter().find(|(c, _)| *c == v.case).map(|(_, w)| *w);
                    assert!(
                        why.is_some(),
                        "{}: §86 gives line 6b = {statutory} and the vector asserts {:?}. A vector \
                         that contradicts the statute is not a witness. Adjudicate against the FORM \
                         and either correct the vector or add it to DISPUTED with the arithmetic.",
                        v.case,
                        v.taxable_social_security_6b
                    );
                    carried += 1;
                }
            }
        }
        assert_eq!(
            carried,
            DISPUTED.len(),
            "every DISPUTED entry must name a vector that really disagrees — {} disagree, {} listed",
            carried,
            DISPUTED.len()
        );
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

    /// ★★★ **THE TRIPWIRE, DISCHARGED AND REPLACED 2026-09-20 — the retirement lines now EXIST, so the
    /// condition this test warned about has arrived, and its old assertion would red forever.**
    ///
    /// It used to assert that `Form1040Lines` declared NO retirement line, with the message *"wire them
    /// into a KAT that drives each case and compares the printed lines, then delete this test."* T14.5
    /// added `line4a`/`line4b`/`line5a`/`line5b`, so it fired exactly as intended.
    ///
    /// ★★ **But it must not simply be deleted, because its instruction CANNOT yet be followed and that
    /// is a fact about the corpus, not a choice.** These vectors carry expected OUTPUTS only; this
    /// module's own header says *"The raw data is **not** committed here"*. Constructing inputs that
    /// reproduce the expected lines would derive the expectation from the thing under test — FR-230's
    /// defect exactly, a kill that measures nothing. Driving them needs the MeF→`ReturnInputs`
    /// translator, which is **FR-255 step 2** and is unbuilt.
    ///
    /// ★★★ So the pressure moves rather than lifting. This asserts the two things a published crate can
    /// check; the third — **if someone closes FR-255 without wiring these vectors, a test reds** — lives
    /// in `xtask`, because reading `FOLLOWUPS.md` from here needs an `include_str!` that escapes the
    /// crate root and ships a broken tarball. See the note at item (3).
    #[test]
    fn the_vectors_are_unconsumed_and_the_ledger_says_which_task_consumes_them() {
        // (1) The retirement lines exist — so "waiting for the lines" is no longer the reason.
        let printed = include_str!("printed.rs");
        let i = printed
            .find("pub struct Form1040Lines")
            .expect("Form1040Lines is declared in printed.rs");
        let j = printed[i..].find("\n}").expect("the struct closes") + i;
        assert!(
            first_retirement_line_declared(&printed[i..j]).is_some(),
            "the retirement lines have DISAPPEARED from Form1040Lines — this test's premise is gone, \
             and the earlier tripwire (assert none is declared) is the one that belongs here again"
        );

        // (2) Every vector is internally consistent, which is all that can be checked without inputs:
        //     4b + 5b + 6b + the other components must equal the corpus's own line 9.
        for v in PUBLIC_RETIREMENT_VECTORS {
            assert!(
                v.line9_reconciles(),
                "{}: the transcribed lines do not sum to the corpus's own line 9 — a transcription \
                 error in this module, not a defect in btctax",
                v.case
            );
        }

        // (3) ★★★ THE TEETH LIVE IN `xtask`, NOT HERE — and the reason is a publish trap this repo has
        //     already been bitten by. The first version of this test read the ledger with an
        //     `include_str!` reaching four directories UP, out of this crate — which
        //     `repo_hygiene::no_published_crate_includes_a_file_outside_its_own_root` refused: such an
        //     include ships a BROKEN crates.io tarball and the publish still exits 0.
        //
        //     ★ The literal path is deliberately NOT written here: that hygiene check greps the source,
        //       so quoting the offending string in a comment would red it again.
        //
        //     So `xtask::ledger_check`'s `fr255_step_2_is_open_while_these_vectors_are_unconsumed`
        //     carries it: the crate that already reads `FOLLOWUPS.md` is the one that should.
        assert!(
            !PUBLIC_RETIREMENT_VECTORS.is_empty(),
            "a census over nothing passes"
        );

        // ★★ B1 — the predicate observed discriminating, on fixtures, because the real plant cannot be
        //    run: adding or removing a field on `Form1040Lines` is an E0063/E0027 at every construction
        //    site and the crate stops compiling before any test executes.
        assert_eq!(
            first_retirement_line_declared("    pub line1z: Usd,\n    pub line4b: Usd,\n"),
            Some("pub line4b:"),
            "a declared retirement line must be found — otherwise this predicate is decorative"
        );
        assert_eq!(
            first_retirement_line_declared("    pub line1z: Usd,\n    pub line7: Usd,\n"),
            None,
            "and a body with no retirement line must not be flagged"
        );
    }
}
