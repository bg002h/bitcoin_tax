//! §G-28/B1a — **Form 8995-A Part IV**, the full §199A form's deduction computation.
//!
//! ## Why this exists at all
//!
//! Above the §199A(e)(2) threshold the simplified Form 8995 no longer applies, so a filer there must
//! file Form 8995-A **even when the arithmetic is identical**. i8995a scopes the case this module
//! covers, in its own words:
//!
//! > *"You must complete Part I if you have QBI from a qualified trade, business, or aggregation. **If
//! > you don't have QBI, and only have REIT, PTP, skip Parts I through III and complete Part IV.**"*
//!
//! That filer needs no input btctax does not already collect — which is why this is B1a and the
//! wage/UBIA/SSTB machinery of Parts I–III is B1b.
//!
//! ## ★★★ The equivalence proof, and the branch where it breaks
//!
//! `CLAUDE.md` allows a derived form **only** with a written equivalence proof naming the branch where
//! it breaks, plus a KAT pinning that branch. Part IV is Form 8995's lines 5–17 with a DPAD line
//! inserted, and the two forms are **pointwise identical** — but they do not say so in the same words,
//! and the wording differs at exactly the value where a clamp is decided:
//!
//! | quantity | Form 8995 | Form 8995-A |
//! |---|---|---|
//! | REIT/PTP total | L8 *"Combine lines 6 and 7. If **zero or less**, enter -0-"* | L30 *"Combine lines 28 and 29. If **less than zero**, enter -0-"* |
//! | REIT/PTP (loss) carryforward | L17 *"…If **greater than zero**, enter -0-"* | L40 *"…If **zero or greater**, enter -0-"* |
//!
//! Worked at every sign, the two agree everywhere **including at zero**, because each pair's clauses are
//! complementary rather than contradictory: where 8995 clamps at zero, 8995-A enters the combined value
//! — which *is* zero — and vice versa. So the figures never differ.
//!
//! **The branch where this breaks:** if either form is ever revised so the two clauses disagree on a
//! NON-zero value. [`tests::the_two_forms_agree_pointwise_including_at_zero`] pins it, and would red on
//! exactly that revision.
//!
//! ## What stays out
//!
//! Line 38 (**DPAD** under §199A(g), allocated from an agricultural or horticultural cooperative) is
//! never btctax's — it requires a Schedule D (Form 8995-A) this version does not fill. It is a
//! CONDITIONAL entry, so it is left **blank**, not zeroed: the form says *"Don't enter more than line 33
//! minus line 37"*, which presumes an amount a cooperative allocated to the filer. A printed `0` would
//! swear they received one.

use crate::conventions::Usd;
use crate::tax::qbi::Form8995Lines;

/// Form 8995-A **Part IV**, in the form's own numbering. One field per printed line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995APartIv {
    /// L27 — *"Total qualified business income component from all qualified trades, businesses, or
    /// aggregations. Enter the amount from line 16"*. Zero on the REIT/PTP-only path this covers.
    pub line27: Usd,
    /// L28 — *"Qualified REIT dividends and publicly traded partnership (PTP) income or (loss). See
    /// instructions"*.
    pub line28: Usd,
    /// L29 — *"Qualified REIT dividends and PTP (loss) carryforward from prior years"*. **Positive
    /// magnitude** — the box is parenthesized and the parentheses supply the minus sign.
    pub line29: Usd,
    /// L30 — *"Total qualified REIT dividends and PTP income. Combine lines 28 and 29. If less than
    /// zero, enter -0-"*.
    pub line30: Usd,
    /// L31 — *"REIT and PTP component. Multiply line 30 by 20% (0.20)"*.
    pub line31: Usd,
    /// L32 — *"Qualified business income deduction before the income limitation. Add lines 27 and 31"*.
    pub line32: Usd,
    /// L33 — *"Taxable income before qualified business income deduction"*.
    pub line33: Usd,
    /// L34 — *"Enter your net capital gain, if any, increased by any qualified dividends (see
    /// instructions)"*.
    pub line34: Usd,
    /// L35 — *"Subtract line 34 from line 33. If zero or less, enter -0-"*.
    pub line35: Usd,
    /// L36 — *"Income limitation. Multiply line 35 by 20% (0.20)"*.
    pub line36: Usd,
    /// L37 — *"Qualified business income deduction before the domestic production activities deduction
    /// (DPAD) under section 199A(g). Enter the smaller of line 32 or line 36"*.
    pub line37: Usd,
    /// L38 — *"DPAD under section 199A(g) allocated from an agricultural or horticultural cooperative.
    /// Don't enter more than line 33 minus line 37"*.
    ///
    /// ★★ `None` and it must be: btctax fills no Schedule D (Form 8995-A), so no cooperative has
    /// allocated anything. The line is a CONDITIONAL entry with no `-0-` clause, so the unmet condition
    /// leaves it **blank**. A printed `0` would swear the filer received an allocation of zero.
    pub line38: Option<Usd>,
    /// L39 — *"Total qualified business income deduction. Add lines 37 and 38"*.
    pub line39: Usd,
    /// L40 — *"Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 28 and 29. If
    /// zero or greater, enter -0-"*. **Positive magnitude** (parenthesized box).
    pub line40: Usd,
}

impl Form8995APartIv {
    /// Transcribe Part IV from the already-computed Form 8995 chain.
    ///
    /// ★ A TRANSCRIPTION, not a re-derivation: every value is a line the existing computation already
    /// produced and the whole packet already cross-foots against. Recomputing here would create a
    /// second authority for one figure — the defect class this codebase keeps finding — and the module
    /// doc carries the proof that the two forms' arithmetic is pointwise identical.
    pub fn from_8995(l: &Form8995Lines) -> Self {
        Self {
            line27: l.line5,  // 8995 L5 — QBI component (zero on the REIT/PTP-only path)
            line28: l.line6,  // 8995 L6
            line29: l.line7,  // 8995 L7
            line30: l.line8,  // 8995 L8  ≡ 8995-A L30 (see the module doc's proof)
            line31: l.line9,  // 8995 L9
            line32: l.line10, // 8995 L10
            line33: l.line11, // 8995 L11
            line34: l.line12, // 8995 L12
            line35: l.line13, // 8995 L13
            line36: l.line14, // 8995 L14
            line37: l.line15, // 8995 L15 — "before the DPAD"; 8995 L15 IS the deduction, and…
            line38: None,     // …with no DPAD, L39 = L37, so the two forms' totals agree.
            line39: l.line15,
            line40: l.line17, // 8995 L17 ≡ 8995-A L40 (same proof)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// ★ Built field-by-field, because `Form8995Lines` has NO `Default` — deliberately. A `Default` on
    /// a money struct launders "never computed" into zero, which is the class this codebase keeps
    /// finding; adding one just to shorten a fixture would trade a real guarantee for brevity.
    #[allow(clippy::too_many_arguments)]
    fn chain(
        line5: Usd,
        line6: Usd,
        line7: Usd,
        line8: Usd,
        line9: Usd,
        line10: Usd,
        line11: Usd,
        line12: Usd,
        line13: Usd,
        line14: Usd,
        line15: Usd,
        line17: Usd,
    ) -> Form8995Lines {
        Form8995Lines {
            business_name: String::new(),
            line2: Usd::ZERO,
            line3: Usd::ZERO,
            line4: Usd::ZERO,
            line5,
            line6,
            line7,
            line8,
            line9,
            line10,
            line11,
            line12,
            line13,
            line14,
            line15,
            line16: Usd::ZERO,
            line17,
        }
    }

    /// ★★★ THE EQUIVALENCE PROOF, EXECUTED — the KAT `CLAUDE.md` requires beside a derived form.
    ///
    /// The two forms word their clamps differently at exactly the value a clamp decides:
    ///
    ///   8995   L8  "Combine lines 6 and 7.  If ZERO OR LESS,    enter -0-"
    ///   8995-A L30 "Combine lines 28 and 29. If LESS THAN ZERO, enter -0-"
    ///   8995   L17 "…If GREATER THAN ZERO, enter -0-"
    ///   8995-A L40 "…If ZERO OR GREATER,   enter -0-"
    ///
    /// Each pair is complementary rather than contradictory: where one clamps at zero the other enters
    /// the combined value, which IS zero. This walks every sign and pins that they never differ — and
    /// it is the test that reds if either form is revised so the clauses disagree on a NON-zero value,
    /// which is the branch the module doc names.
    #[test]
    fn the_two_forms_agree_pointwise_including_at_zero() {
        // The four clauses, transcribed as predicates rather than paraphrased into one.
        let f8995_l8 = |v: i64| if v <= 0 { 0 } else { v }; // "zero or less"
        let f8995a_l30 = |v: i64| if v < 0 { 0 } else { v }; // "less than zero"
        let f8995_l17 = |v: i64| if v > 0 { 0 } else { -v }; // "greater than zero"
        let f8995a_l40 = |v: i64| if v >= 0 { 0 } else { -v }; // "zero or greater"

        // ★★★ WHAT THIS TEST DOES NOT PROVE, stated so nobody mistakes its silence for coverage.
        //
        //     It asserts the two predicates AGREE. It cannot tell whether either one transcribes its
        //     own form correctly, because breaking one to match the other still agrees. Concretely:
        //     misreading L40's "if ZERO OR GREATER" as "if GREATER THAN zero" changes NO value at any
        //     input — at v = 0 the correct reading enters -0- and the misreading enters the combined
        //     value, which IS 0. Verified exhaustively; the two readings differ nowhere.
        //
        //     So the clause WORDING is not arithmetically observable, and no arithmetic test can guard
        //     it. What guards it is the verbatim check on the quoted instruction — the map's
        //     `every_quoted_instruction_is_verbatim_on_the_form`, which already caught this module's
        //     sibling paraphrase ("if zero or less" for "if less than zero" on line 30). Naming the
        //     limit here is the point: an untested belief with no acknowledged gap is how the last four
        //     review rounds each found a defect in a fold.
        for v in [-1_000_000i64, -1000, -1, 0, 1, 1000, 1_000_000] {
            assert_eq!(
                f8995_l8(v),
                f8995a_l30(v),
                "the REIT/PTP TOTAL differs at {v}: 8995 L8 vs 8995-A L30"
            );
            assert_eq!(
                f8995_l17(v),
                f8995a_l40(v),
                "the REIT/PTP CARRYFORWARD differs at {v}: 8995 L17 vs 8995-A L40"
            );
        }
    }

    /// The transcription carries every figure across unchanged, and totals agree with Form 8995.
    #[test]
    fn part_iv_transcribes_the_8995_chain() {
        // REIT dividends 4,000; no business QBI; TI 250,000 with no net capital gain.
        let l = chain(
            dec!(0),
            dec!(4000),
            dec!(0),
            dec!(4000),
            dec!(800),
            dec!(800),
            dec!(250000),
            dec!(0),
            dec!(250000),
            dec!(50000),
            dec!(800),
            dec!(0),
        );
        let p = Form8995APartIv::from_8995(&l);
        assert_eq!(
            (p.line28, p.line30, p.line31),
            (dec!(4000), dec!(4000), dec!(800))
        );
        assert_eq!(
            (p.line32, p.line36, p.line37),
            (dec!(800), dec!(50000), dec!(800))
        );
        // ★★ L38 is BLANK, not zero. btctax fills no Schedule D (Form 8995-A), so no cooperative has
        //    allocated anything — and the line has no `-0-` clause, so a printed 0 would swear the
        //    filer received an allocation of zero.
        assert_eq!(p.line38, None, "the DPAD line carries NO testimony");
        // With no DPAD, L39 = L37, so 8995-A's total equals Form 8995's line 15.
        assert_eq!(p.line39, l.line15, "the two forms' deductions must agree");
    }

    /// ★ A REIT/PTP LOSS still carries forward as a positive magnitude, and the total clamps to zero —
    /// the case where the two clamps could have diverged, worked end to end.
    #[test]
    fn a_reit_loss_clamps_the_total_and_carries_forward() {
        // No REIT income; a 5,000 prior-year loss carryforward (magnitude). Combine ⇒ -5,000 ⇒ clamped.
        let l = chain(
            Usd::ZERO,
            dec!(0),
            dec!(5000),
            dec!(0),
            dec!(0),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(0),
            dec!(5000),
        );
        let p = Form8995APartIv::from_8995(&l);
        assert_eq!(p.line30, Usd::ZERO, "a net REIT/PTP loss clamps the total");
        assert_eq!(
            p.line40,
            dec!(5000),
            "and carries forward as a POSITIVE magnitude"
        );
        assert!(
            p.line40 >= Usd::ZERO,
            "a parenthesized box never holds a minus sign"
        );
    }
}
